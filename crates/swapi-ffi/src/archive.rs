use super::*;
use swapi_core::{ArchiveCache, ArchiveStore};

#[derive(Clone, Debug, Default, PartialEq, uniffi::Record)]
pub struct CacheStatus {
    pub saved_at: Option<u64>,
    pub from_cache: bool,
    pub offline: bool,
    pub refresh_failed: bool,
    pub saving_failed: bool,
}
impl From<swapi_core::CacheInfo> for CacheStatus {
    fn from(value: swapi_core::CacheInfo) -> Self {
        Self {
            saved_at: value.saved_at,
            from_cache: value.from_cache,
            offline: value.offline,
            refresh_failed: value.refresh_failed,
            saving_failed: value.saving_failed,
        }
    }
}
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SavedEntry {
    pub category: CatalogCategory,
    pub entry: CatalogEntry,
    pub saved_at: u64,
    pub favorite: bool,
}
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct FavoriteIdentity {
    pub category: CatalogCategory,
    pub id: u32,
}
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SavedCollection {
    pub category: CatalogCategory,
    pub saved_count: u32,
    pub total_count: Option<u32>,
    pub complete: bool,
    pub downloaded_at: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum DownloadPhase {
    Idle,
    Downloading {
        category: CatalogCategory,
        completed: u32,
        total: u32,
    },
    Ready {
        category: CatalogCategory,
        count: u32,
    },
    Failed {
        category: CatalogCategory,
        kind: FailureKind,
    },
}
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ArchiveSnapshot {
    pub revision: u64,
    pub query: String,
    pub favorites_only: bool,
    pub offline: bool,
    pub entries: Vec<SavedEntry>,
    pub favorites: Vec<FavoriteIdentity>,
    pub collections: Vec<SavedCollection>,
    pub download: DownloadPhase,
    pub error: Option<FailureKind>,
}
impl From<swapi_core::ArchiveSnapshot> for ArchiveSnapshot {
    fn from(value: swapi_core::ArchiveSnapshot) -> Self {
        Self {
            revision: value.revision,
            query: value.query,
            favorites_only: value.favorites_only,
            offline: value.offline,
            entries: value
                .items
                .into_iter()
                .map(|s| SavedEntry {
                    category: s.record.kind.into(),
                    entry: s.record.into(),
                    saved_at: s.saved_at,
                    favorite: s.favorite,
                })
                .collect(),
            favorites: value
                .favorites
                .into_iter()
                .map(|f| FavoriteIdentity {
                    category: f.kind.into(),
                    id: f.id,
                })
                .collect(),
            collections: value
                .collections
                .into_iter()
                .map(|c| SavedCollection {
                    category: c.kind.into(),
                    saved_count: c.saved,
                    total_count: c.total,
                    complete: c.complete,
                    downloaded_at: c.downloaded_at,
                })
                .collect(),
            download: match value.download {
                swapi_core::DownloadState::Idle => DownloadPhase::Idle,
                swapi_core::DownloadState::Running { kind, saved, total } => {
                    DownloadPhase::Downloading {
                        category: kind.into(),
                        completed: saved,
                        total,
                    }
                }
                swapi_core::DownloadState::Complete { kind, count } => DownloadPhase::Ready {
                    category: kind.into(),
                    count,
                },
                swapi_core::DownloadState::Failed { kind, error } => DownloadPhase::Failed {
                    category: kind.into(),
                    kind: error.into(),
                },
            },
            error: value.error.map(Into::into),
        }
    }
}
#[uniffi::export(with_foreign)]
pub trait ArchiveListener: Send + Sync {
    fn on_state(&self, snapshot: ArchiveSnapshot) -> Result<(), BridgeError>;
}
#[derive(uniffi::Object)]
pub struct ArchiveSession {
    client: SwapiClient,
    store: ArchiveStore,
    closed: AtomicBool,
}
impl ArchiveSession {
    fn open(client: SwapiClient, storage_path: String) -> Result<Arc<Self>, BridgeError> {
        let cache = Arc::new(
            ArchiveCache::open(storage_path, client.endpoint())
                .map_err(|_| BridgeError::Storage)?,
        );
        let client = client.with_cache(cache.clone());
        let store = ArchiveStore::new(client.clone(), cache, runtime().handle());
        Ok(Arc::new(Self {
            client,
            store,
            closed: AtomicBool::new(false),
        }))
    }
    fn ensure_open(&self) -> Result<(), BridgeError> {
        if self.closed.load(Ordering::Acquire) {
            Err(BridgeError::Closed)
        } else {
            Ok(())
        }
    }
}
#[uniffi::export]
impl ArchiveSession {
    #[uniffi::constructor]
    pub fn new(storage_path: String) -> Result<Arc<Self>, BridgeError> {
        Self::open(
            SwapiClient::new().map_err(|_| BridgeError::Initialization)?,
            storage_path,
        )
    }
    #[uniffi::constructor]
    pub fn with_endpoint(storage_path: String, base_url: String) -> Result<Arc<Self>, BridgeError> {
        Self::open(
            SwapiClient::with_endpoint(&base_url, std::time::Duration::from_secs(15))
                .map_err(|_| BridgeError::Initialization)?,
            storage_path,
        )
    }
    pub fn people_session(&self) -> Result<Arc<PeopleSession>, BridgeError> {
        self.ensure_open()?;
        Ok(PeopleSession::with_client(self.client.clone()))
    }
    pub fn catalog_session(
        &self,
        category: CatalogCategory,
        record_id: Option<u32>,
    ) -> Result<Arc<CatalogSession>, BridgeError> {
        self.ensure_open()?;
        Ok(CatalogSession::with_client(
            self.client.clone(),
            category,
            record_id,
        ))
    }
    pub fn character_session(&self) -> Result<Arc<CharacterSession>, BridgeError> {
        self.ensure_open()?;
        Ok(CharacterSession::with_client(self.client.clone()))
    }
    pub fn search(&self, query: String) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.search(query);
        Ok(())
    }
    pub fn set_favorites_only(&self, value: bool) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.favorites_only(value);
        Ok(())
    }
    pub fn set_offline(&self, value: bool) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.offline(value);
        Ok(())
    }
    pub fn set_favorite(
        &self,
        category: CatalogCategory,
        id: u32,
        value: bool,
    ) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.favorite(category.into(), id, value);
        Ok(())
    }
    pub fn download(&self, category: CatalogCategory) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.download(category.into());
        Ok(())
    }
    pub fn cancel_download(&self) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.cancel_download();
        Ok(())
    }
    pub fn clear_cache(&self) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.clear_cache();
        Ok(())
    }
    pub fn shutdown(&self) {
        self.closed.store(true, Ordering::Release);
        self.store.close();
    }
    pub fn subscribe(
        &self,
        listener: Arc<dyn ArchiveListener>,
    ) -> Result<Arc<Subscription>, BridgeError> {
        self.ensure_open()?;
        let mut states = self.store.observe();
        let cancelled = Arc::new(AtomicBool::new(false));
        let guard = cancelled.clone();
        let task = runtime()
            .spawn(async move {
                loop {
                    let snapshot = states.borrow_and_update().clone().into();
                    if guard.load(Ordering::Acquire) || listener.on_state(snapshot).is_err() {
                        break;
                    }
                    if states.changed().await.is_err() {
                        break;
                    }
                }
            })
            .abort_handle();
        Ok(Arc::new(Subscription { cancelled, task }))
    }
}
