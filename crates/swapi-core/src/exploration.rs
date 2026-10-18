//! Discovery and comparison policy. Native layers only render these projections.
use crate::cache::storage;
use crate::{
    ApiError, ArchiveCache, CatalogRecord, Measurement, ResourceKind, ResourceRef, SavedRecord,
};
use rusqlite::params;

#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonValue {
    pub display: String,
    pub fraction: Option<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonMetric {
    pub label: String,
    pub values: Vec<ComparisonValue>,
    pub lower_is_better: bool,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Exploration {
    pub featured: Option<SavedRecord>,
    pub recent: Vec<SavedRecord>,
    pub suggestions: Vec<SavedRecord>,
    pub ships: Vec<CatalogRecord>,
    pub metrics: Vec<ComparisonMetric>,
    pub widget: Vec<SavedRecord>,
}

pub fn compare_ships(ships: &[CatalogRecord]) -> Vec<ComparisonMetric> {
    ["Length (m)", "Crew", "Cargo (kg)", "Hyperdrive rating"]
        .into_iter()
        .map(|label| {
            let raw: Vec<_> = ships
                .iter()
                .map(|ship| {
                    ship.facts
                        .iter()
                        .find(|f| f.label == label)
                        .map(|f| f.value.as_str())
                        .unwrap_or("unknown")
                })
                .collect();
            let numeric: Vec<_> = raw
                .iter()
                .map(|value| match Measurement::parse(value) {
                    Ok(Measurement::Known(number)) => Some(number),
                    _ => None,
                })
                .collect();
            let max = numeric.iter().flatten().copied().fold(0.0_f64, f64::max);
            ComparisonMetric {
                label: label.into(),
                lower_is_better: label == "Hyperdrive rating",
                values: raw
                    .into_iter()
                    .zip(numeric)
                    .map(|(value, number)| ComparisonValue {
                        display: match value {
                            "unknown" => "Unknown".into(),
                            "n/a" => "Not applicable".into(),
                            _ => value.into(),
                        },
                        fraction: number.map(|n| if max > 0.0 { n / max } else { 0.0 }),
                    })
                    .collect(),
            }
        })
        .collect()
}

impl ArchiveCache {
    pub fn visit(&self, kind: ResourceKind, id: u32) -> Result<(), ApiError> {
        let mut connection = self.connection.lock().map_err(|_| ApiError::Storage)?;
        let tx = connection.transaction().map_err(storage)?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM records WHERE namespace=?1 AND category=?2 AND id=?3)",
                params![self.namespace, kind.path(), id],
                |r| r.get(0),
            )
            .map_err(storage)?;
        if !exists {
            return Err(ApiError::InvalidRequest);
        }
        tx.execute("INSERT INTO visits(namespace,category,id,sequence) VALUES(?1,?2,?3,(SELECT COALESCE(MAX(sequence),0)+1 FROM visits WHERE namespace=?1)) ON CONFLICT(namespace,category,id) DO UPDATE SET sequence=excluded.sequence", params![self.namespace,kind.path(),id]).map_err(storage)?;
        tx.execute("DELETE FROM visits WHERE namespace=?1 AND rowid NOT IN (SELECT rowid FROM visits WHERE namespace=?1 ORDER BY sequence DESC LIMIT 20)", [&self.namespace]).map_err(storage)?;
        tx.commit().map_err(storage)?;
        drop(connection);
        self.changed();
        Ok(())
    }
    pub fn select_ship(&self, id: u32, selected: bool) -> Result<(), ApiError> {
        let mut connection = self.connection.lock().map_err(|_| ApiError::Storage)?;
        let tx = connection.transaction().map_err(storage)?;
        if selected {
            let count: u32 = tx
                .query_row(
                    "SELECT COUNT(*) FROM comparison WHERE namespace=?1 AND id<>?2",
                    params![self.namespace, id],
                    |r| r.get(0),
                )
                .map_err(storage)?;
            let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM records WHERE namespace=?1 AND category='starships' AND id=?2)", params![self.namespace,id], |r| r.get(0)).map_err(storage)?;
            if count >= 2 || !exists {
                return Err(ApiError::InvalidRequest);
            }
            tx.execute("INSERT OR IGNORE INTO comparison(namespace,id,sequence) VALUES(?1,?2,(SELECT COALESCE(MAX(sequence),0)+1 FROM comparison WHERE namespace=?1))", params![self.namespace,id]).map_err(storage)?;
        } else {
            tx.execute(
                "DELETE FROM comparison WHERE namespace=?1 AND id=?2",
                params![self.namespace, id],
            )
            .map_err(storage)?;
        }
        tx.commit().map_err(storage)?;
        drop(connection);
        self.changed();
        Ok(())
    }
    pub fn exploration(&self, day: u64) -> Result<Exploration, ApiError> {
        let records = self.records("", false)?;
        let connection = self.connection.lock().map_err(|_| ApiError::Storage)?;
        let mut statement = connection
            .prepare("SELECT category,id FROM visits WHERE namespace=?1 ORDER BY sequence DESC")
            .map_err(storage)?;
        let visits = statement
            .query_map([&self.namespace], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))
            })
            .map_err(storage)?;
        let mut recent = Vec::new();
        for visit in visits {
            let (kind, id) = visit.map_err(storage)?;
            if let Some(record) = records
                .iter()
                .find(|s| s.record.kind.path() == kind && s.record.id == id)
            {
                recent.push(record.clone());
            }
        }
        let mut statement = connection
            .prepare("SELECT id FROM comparison WHERE namespace=?1 ORDER BY sequence")
            .map_err(storage)?;
        let ids = statement
            .query_map([&self.namespace], |r| r.get::<_, u32>(0))
            .map_err(storage)?;
        let mut ships = Vec::new();
        for id in ids {
            let id = id.map_err(storage)?;
            if let Some(record) = records
                .iter()
                .find(|s| s.record.kind == ResourceKind::Starship && s.record.id == id)
            {
                ships.push(record.record.clone());
            }
        }
        let worlds: Vec<_> = records
            .iter()
            .filter(|s| s.record.kind == ResourceKind::Planet)
            .collect();
        let featured = if worlds.is_empty() {
            None
        } else {
            Some(worlds[(day % worlds.len() as u64) as usize].clone())
        };
        let mut suggestions = Vec::new();
        if let Some(last) = recent.first() {
            for group in &last.record.related {
                for id in &group.ids {
                    if let Some(record) = records
                        .iter()
                        .find(|s| s.record.kind == group.kind && s.record.id == *id)
                        && !recent.iter().any(|s| same(s, record))
                        && !suggestions.iter().any(|s| same(s, record))
                    {
                        suggestions.push(record.clone());
                    }
                }
            }
        }
        for record in &records {
            if suggestions.len() >= 6 {
                break;
            }
            if !recent.iter().any(|s| same(s, record))
                && !suggestions.iter().any(|s| same(s, record))
                && !featured.as_ref().is_some_and(|s| same(s, record))
            {
                suggestions.push(record.clone());
            }
        }
        suggestions.truncate(6);
        recent.truncate(8);
        let mut widget: Vec<_> = records
            .iter()
            .filter(|s| s.favorite)
            .take(1)
            .cloned()
            .collect();
        if let Some(record) = &featured
            && !widget.iter().any(|s| same(s, record))
        {
            widget.push(record.clone());
        }
        if widget.is_empty() {
            widget.extend(records.first().cloned());
        }
        Ok(Exploration {
            featured,
            recent,
            suggestions,
            metrics: compare_ships(&ships),
            ships,
            widget,
        })
    }
}
fn same(a: &SavedRecord, b: &SavedRecord) -> bool {
    a.record.kind == b.record.kind && a.record.id == b.record.id
}

/// Widget routes are a small allowlisted grammar, never arbitrary fetch URLs.
pub fn parse_record_link(value: &str) -> Result<ResourceRef, ApiError> {
    let url = url::Url::parse(value).map_err(|_| ApiError::InvalidRequest)?;
    if url.scheme() != "holocron"
        || url.host_str() != Some("record")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ApiError::InvalidRequest);
    }
    let parts: Vec<_> = url.path().trim_start_matches('/').split('/').collect();
    if parts.len() != 2 || parts[1].is_empty() || !parts[1].bytes().all(|b| b.is_ascii_digit()) {
        return Err(ApiError::InvalidRequest);
    }
    let kind = ResourceKind::from_path(parts[0]).map_err(|_| ApiError::InvalidRequest)?;
    let id = parts[1]
        .parse::<u32>()
        .map_err(|_| ApiError::InvalidRequest)?;
    if id == 0 {
        return Err(ApiError::InvalidRequest);
    }
    Ok(ResourceRef { kind, id })
}
