//! Persistent, namespace-isolated archive. Only validated API records are saved.
use crate::{ApiError, CatalogRecord, ResourceKind};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use std::{
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;
use url::Url;

pub const FRESH_SECONDS: u64 = 24 * 60 * 60;
const RESPONSE_BUDGET: i64 = 16 * 1024 * 1024;

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CacheInfo {
    pub saved_at: Option<u64>,
    pub from_cache: bool,
    pub offline: bool,
    pub refresh_failed: bool,
    pub saving_failed: bool,
}

#[derive(Clone, Debug)]
pub struct Loaded<T> {
    pub value: T,
    pub cache: CacheInfo,
}
impl<T> Loaded<T> {
    pub fn live(value: T) -> Self {
        Self {
            value,
            cache: CacheInfo::default(),
        }
    }
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Loaded<U> {
        Loaded {
            value: f(self.value),
            cache: self.cache,
        }
    }
}

pub(crate) struct CachedResponse {
    pub body: Value,
    pub saved_at: u64,
}
impl CachedResponse {
    pub fn fresh(&self) -> bool {
        now().saturating_sub(self.saved_at) < FRESH_SECONDS
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SavedRecord {
    pub record: CatalogRecord,
    pub saved_at: u64,
    pub favorite: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollectionInfo {
    pub kind: ResourceKind,
    pub saved: u32,
    pub total: Option<u32>,
    pub complete: bool,
    pub downloaded_at: Option<u64>,
}

pub struct ArchiveCache {
    connection: Mutex<Connection>,
    namespace: String,
    offline: AtomicBool,
    changes: watch::Sender<u64>,
}

fn storage(_: rusqlite::Error) -> ApiError {
    ApiError::Storage
}

impl ArchiveCache {
    pub fn open(path: impl AsRef<Path>, namespace: &str) -> Result<Self, ApiError> {
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(storage)?;
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(storage)?;
        if version > 1 {
            return Err(ApiError::Storage);
        }
        connection.execute_batch("PRAGMA journal_mode=WAL;
            BEGIN;
            CREATE TABLE IF NOT EXISTS responses (
                namespace TEXT NOT NULL, url TEXT NOT NULL, body TEXT NOT NULL, saved_at INTEGER NOT NULL,
                PRIMARY KEY(namespace, url));
            CREATE TABLE IF NOT EXISTS records (
                namespace TEXT NOT NULL, category TEXT NOT NULL, id INTEGER NOT NULL,
                body TEXT NOT NULL, saved_at INTEGER NOT NULL, favorite INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY(namespace, category, id));
            CREATE TABLE IF NOT EXISTS collections (
                namespace TEXT NOT NULL, category TEXT NOT NULL, total INTEGER,
                complete INTEGER NOT NULL DEFAULT 0, downloaded_at INTEGER,
                PRIMARY KEY(namespace, category));
            CREATE TABLE IF NOT EXISTS preferences (
                namespace TEXT PRIMARY KEY, offline INTEGER NOT NULL DEFAULT 0);
            PRAGMA user_version=1;
            COMMIT;").map_err(storage)?;
        let offline = connection
            .query_row(
                "SELECT offline FROM preferences WHERE namespace=?1",
                [namespace],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?
            .unwrap_or(false);
        let (changes, _) = watch::channel(0);
        Ok(Self {
            connection: Mutex::new(connection),
            namespace: namespace.into(),
            offline: AtomicBool::new(offline),
            changes,
        })
    }

    pub fn observe(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }
    fn changed(&self) {
        self.changes.send_modify(|revision| *revision += 1);
    }
    pub fn is_offline(&self) -> bool {
        self.offline.load(Ordering::Acquire)
    }
    pub fn set_offline(&self, value: bool) -> Result<(), ApiError> {
        self.connection.lock().map_err(|_| ApiError::Storage)?.execute(
            "INSERT INTO preferences(namespace,offline) VALUES(?1,?2) ON CONFLICT(namespace) DO UPDATE SET offline=excluded.offline",
            params![self.namespace, value]).map_err(storage)?;
        self.offline.store(value, Ordering::Release);
        self.changed();
        Ok(())
    }

    pub(crate) fn response(&self, url: &Url) -> Result<Option<CachedResponse>, ApiError> {
        let connection = self.connection.lock().map_err(|_| ApiError::Storage)?;
        let mut raw: Option<(String, u64)> = connection
            .query_row(
                "SELECT body,saved_at FROM responses WHERE namespace=?1 AND url=?2",
                params![self.namespace, url.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(storage)?;
        // Collection results contain complete records, so details work offline
        // even when the user has never opened that record's detail screen.
        // The normalized record is authoritative after a newer collection refresh.
        if url.query().is_none() {
            let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
            if let ["api", category, id] = parts.as_slice()
                && let Ok(id) = id.parse::<u32>()
            {
                raw = connection.query_row("SELECT body,saved_at FROM records WHERE namespace=?1 AND category=?2 AND id=?3",
                        params![self.namespace, category, id], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(storage)?.or(raw);
            }
        }
        raw.map(|(body, saved_at)| {
            Ok(CachedResponse {
                body: serde_json::from_str(&body).map_err(|_| ApiError::Storage)?,
                saved_at,
            })
        })
        .transpose()
    }

    pub(crate) fn save_response(&self, url: &Url, body: &Value) -> Result<u64, ApiError> {
        let kind = ResourceKind::from_path(
            url.path()
                .trim_matches('/')
                .split('/')
                .nth(1)
                .ok_or(ApiError::InvalidData)?,
        )?;
        let values = body
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(|| vec![body.clone()]);
        let records = values
            .into_iter()
            .map(|value| Ok((CatalogRecord::parse(kind, value.clone())?.id, value)))
            .collect::<Result<Vec<_>, ApiError>>()?;
        let saved_at = now();
        let mut connection = self.connection.lock().map_err(|_| ApiError::Storage)?;
        let tx = connection.transaction().map_err(storage)?;
        tx.execute(
            "INSERT INTO responses(namespace,url,body,saved_at) VALUES(?1,?2,?3,?4)
            ON CONFLICT(namespace,url) DO UPDATE SET body=excluded.body,saved_at=excluded.saved_at",
            params![self.namespace, url.as_str(), body.to_string(), saved_at],
        )
        .map_err(storage)?;
        for (id, value) in records {
            tx.execute("INSERT INTO records(namespace,category,id,body,saved_at) VALUES(?1,?2,?3,?4,?5)
                ON CONFLICT(namespace,category,id) DO UPDATE SET body=excluded.body,saved_at=excluded.saved_at",
                params![self.namespace, kind.path(), id, value.to_string(), saved_at]).map_err(storage)?;
        }
        if !url.query_pairs().any(|(key, _)| key == "search")
            && let Some(total) = body.get("count").and_then(Value::as_u64)
        {
            tx.execute("INSERT INTO collections(namespace,category,total) VALUES(?1,?2,?3)
                    ON CONFLICT(namespace,category) DO UPDATE SET total=excluded.total,
                    complete=CASE WHEN collections.total=excluded.total THEN collections.complete ELSE 0 END",
                    params![self.namespace, kind.path(), total]).map_err(storage)?;
        }
        loop {
            let bytes: i64 = tx
                .query_row(
                    "SELECT COALESCE(SUM(length(CAST(body AS BLOB))),0) FROM responses WHERE namespace=?1",
                    [&self.namespace],
                    |r| r.get(0),
                )
                .map_err(storage)?;
            if bytes <= RESPONSE_BUDGET {
                break;
            }
            tx.execute("DELETE FROM responses WHERE rowid=(SELECT rowid FROM responses WHERE namespace=?1 ORDER BY saved_at,rowid LIMIT 1)", [&self.namespace]).map_err(storage)?;
        }
        tx.commit().map_err(storage)?;
        drop(connection);
        self.changed();
        Ok(saved_at)
    }

    pub fn records(&self, query: &str, favorites_only: bool) -> Result<Vec<SavedRecord>, ApiError> {
        let connection = self.connection.lock().map_err(|_| ApiError::Storage)?;
        let mut statement = connection.prepare("SELECT category,body,saved_at,favorite FROM records WHERE namespace=?1 AND (?2=0 OR favorite=1) ORDER BY category,id").map_err(storage)?;
        let rows = statement
            .query_map(params![self.namespace, favorites_only], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, u64>(2)?,
                    r.get::<_, bool>(3)?,
                ))
            })
            .map_err(storage)?;
        let query = query.trim().to_lowercase();
        let mut records = Vec::new();
        for row in rows {
            let (kind, body, saved_at, favorite) = row.map_err(storage)?;
            let record = CatalogRecord::parse(
                ResourceKind::from_path(&kind)?,
                serde_json::from_str(&body).map_err(|_| ApiError::Storage)?,
            )?;
            if query.is_empty()
                || record.name.to_lowercase().contains(&query)
                || record
                    .facts
                    .iter()
                    .any(|f| f.label == "Model" && f.value.to_lowercase().contains(&query))
            {
                records.push(SavedRecord {
                    record,
                    saved_at,
                    favorite,
                });
            }
        }
        records.sort_by_cached_key(|saved| saved.record.name.to_lowercase());
        Ok(records)
    }

    pub fn set_favorite(
        &self,
        kind: ResourceKind,
        id: u32,
        favorite: bool,
    ) -> Result<(), ApiError> {
        let changed = self
            .connection
            .lock()
            .map_err(|_| ApiError::Storage)?
            .execute(
                "UPDATE records SET favorite=?4 WHERE namespace=?1 AND category=?2 AND id=?3",
                params![self.namespace, kind.path(), id, favorite],
            )
            .map_err(storage)?;
        if changed == 0 {
            return Err(ApiError::InvalidRequest);
        }
        self.changed();
        Ok(())
    }

    pub fn collections(&self) -> Result<Vec<CollectionInfo>, ApiError> {
        let connection = self.connection.lock().map_err(|_| ApiError::Storage)?;
        ResourceKind::ALL.into_iter().map(|kind| {
            let saved = connection.query_row("SELECT COUNT(*) FROM records WHERE namespace=?1 AND category=?2", params![self.namespace,kind.path()], |r| r.get(0)).map_err(storage)?;
            let state = connection.query_row("SELECT total,complete,downloaded_at FROM collections WHERE namespace=?1 AND category=?2", params![self.namespace,kind.path()], |r| Ok((r.get::<_, Option<u32>>(0)?,r.get::<_, bool>(1)?,r.get::<_, Option<u64>>(2)?))).optional().map_err(storage)?.unwrap_or((None,false,None));
            Ok(CollectionInfo { kind, saved, total: state.0, complete: state.1, downloaded_at: state.2 })
        }).collect()
    }

    pub(crate) fn mark_download(
        &self,
        kind: ResourceKind,
        total: u32,
        complete: bool,
    ) -> Result<(), ApiError> {
        self.connection.lock().map_err(|_| ApiError::Storage)?.execute(
            "INSERT INTO collections(namespace,category,total,complete,downloaded_at) VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(namespace,category) DO UPDATE SET total=excluded.total,complete=excluded.complete,downloaded_at=excluded.downloaded_at",
            params![self.namespace,kind.path(),total,complete,complete.then(now)]).map_err(storage)?;
        self.changed();
        Ok(())
    }

    pub fn clear_cache(&self) -> Result<(), ApiError> {
        let mut connection = self.connection.lock().map_err(|_| ApiError::Storage)?;
        let tx = connection.transaction().map_err(storage)?;
        tx.execute(
            "DELETE FROM responses WHERE namespace=?1",
            [&self.namespace],
        )
        .map_err(storage)?;
        tx.execute(
            "DELETE FROM records WHERE namespace=?1 AND favorite=0",
            [&self.namespace],
        )
        .map_err(storage)?;
        tx.execute(
            "DELETE FROM collections WHERE namespace=?1",
            [&self.namespace],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
        drop(connection);
        self.changed();
        Ok(())
    }
}
