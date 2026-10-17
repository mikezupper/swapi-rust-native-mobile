use swapi_core::ResourceKind;

#[derive(Clone, Copy, Debug, PartialEq, uniffi::Enum)]
pub enum CatalogCategory {
    People,
    Planets,
    Starships,
    Vehicles,
    Species,
    Films,
}
impl From<CatalogCategory> for ResourceKind {
    fn from(value: CatalogCategory) -> Self {
        match value {
            CatalogCategory::People => Self::Person,
            CatalogCategory::Planets => Self::Planet,
            CatalogCategory::Starships => Self::Starship,
            CatalogCategory::Vehicles => Self::Vehicle,
            CatalogCategory::Species => Self::Species,
            CatalogCategory::Films => Self::Film,
        }
    }
}
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CatalogFact {
    pub label: String,
    pub value: String,
}
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CatalogEntry {
    pub id: u32,
    pub name: String,
    pub facts: Vec<CatalogFact>,
    pub description: Option<String>,
}
impl From<swapi_core::CatalogRecord> for CatalogEntry {
    fn from(record: swapi_core::CatalogRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            description: record.description,
            facts: record
                .facts
                .into_iter()
                .map(|f| CatalogFact {
                    label: f.label,
                    value: f.value,
                })
                .collect(),
        }
    }
}
use super::*;
use swapi_core::{CatalogStore, PeoplePhase as CorePhase};

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum CatalogPhase {
    Idle,
    Loading,
    Ready,
    Refreshing,
    LoadingMore,
    Failed { kind: FailureKind, retryable: bool },
}
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CatalogSnapshot {
    pub revision: u64,
    pub query: String,
    pub items: Vec<CatalogEntry>,
    pub total: u32,
    pub has_more: bool,
    pub phase: CatalogPhase,
}
impl From<swapi_core::CatalogSnapshot> for CatalogSnapshot {
    fn from(value: swapi_core::CatalogSnapshot) -> Self {
        Self {
            revision: value.revision,
            query: value.query,
            items: value.items.into_iter().map(Into::into).collect(),
            total: value.total,
            has_more: value.next_page.is_some(),
            phase: match value.phase {
                CorePhase::Idle => CatalogPhase::Idle,
                CorePhase::Loading => CatalogPhase::Loading,
                CorePhase::Ready => CatalogPhase::Ready,
                CorePhase::Refreshing => CatalogPhase::Refreshing,
                CorePhase::LoadingMore => CatalogPhase::LoadingMore,
                CorePhase::Failed { error, .. } => CatalogPhase::Failed {
                    retryable: error.is_retryable(),
                    kind: error.into(),
                },
            },
        }
    }
}
#[uniffi::export(with_foreign)]
pub trait CatalogListener: Send + Sync {
    fn on_state(&self, snapshot: CatalogSnapshot) -> Result<(), BridgeError>;
}
#[derive(uniffi::Object)]
pub struct CatalogSession {
    store: CatalogStore,
    closed: AtomicBool,
}
impl CatalogSession {
    fn with_client(
        client: SwapiClient,
        category: CatalogCategory,
        record_id: Option<u32>,
    ) -> Arc<Self> {
        Arc::new(Self {
            store: CatalogStore::catalog(client, category.into(), record_id, runtime().handle()),
            closed: AtomicBool::new(false),
        })
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
impl CatalogSession {
    #[uniffi::constructor]
    pub fn new(
        category: CatalogCategory,
        record_id: Option<u32>,
    ) -> Result<Arc<Self>, BridgeError> {
        Ok(Self::with_client(
            SwapiClient::new().map_err(|_| BridgeError::Initialization)?,
            category,
            record_id,
        ))
    }
    #[uniffi::constructor]
    pub fn with_endpoint(
        base_url: String,
        category: CatalogCategory,
        record_id: Option<u32>,
    ) -> Result<Arc<Self>, BridgeError> {
        Ok(Self::with_client(
            SwapiClient::with_endpoint(&base_url, std::time::Duration::from_secs(15))
                .map_err(|_| BridgeError::Initialization)?,
            category,
            record_id,
        ))
    }
    pub fn search(&self, query: String) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.search(query);
        Ok(())
    }
    pub fn refresh(&self) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.refresh();
        Ok(())
    }
    pub fn load_more(&self) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.load_more();
        Ok(())
    }
    pub fn retry(&self) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.retry();
        Ok(())
    }
    pub fn shutdown(&self) {
        self.closed.store(true, Ordering::Release);
        self.store.close();
    }
    pub fn subscribe(
        &self,
        listener: Arc<dyn CatalogListener>,
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
