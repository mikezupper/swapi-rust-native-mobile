use super::*;
use swapi_core::{PeoplePhase as CorePhase, PeopleStore};

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum PeoplePhase {
    Idle,
    Loading,
    Ready,
    Refreshing,
    LoadingMore,
    Failed { kind: FailureKind, retryable: bool },
}
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct PeopleSnapshot {
    pub revision: u64,
    pub query: String,
    pub people: Vec<Character>,
    pub total: u32,
    pub has_more: bool,
    pub phase: PeoplePhase,
}
impl From<swapi_core::PeopleSnapshot> for PeopleSnapshot {
    fn from(value: swapi_core::PeopleSnapshot) -> Self {
        Self {
            revision: value.revision,
            query: value.query,
            people: value.people.into_iter().map(Into::into).collect(),
            total: value.total,
            has_more: value.next_page.is_some(),
            phase: match value.phase {
                CorePhase::Idle => PeoplePhase::Idle,
                CorePhase::Loading => PeoplePhase::Loading,
                CorePhase::Ready => PeoplePhase::Ready,
                CorePhase::Refreshing => PeoplePhase::Refreshing,
                CorePhase::LoadingMore => PeoplePhase::LoadingMore,
                CorePhase::Failed { error, .. } => PeoplePhase::Failed {
                    retryable: error.is_retryable(),
                    kind: error.into(),
                },
            },
        }
    }
}
#[uniffi::export(with_foreign)]
pub trait PeopleListener: Send + Sync {
    fn on_state(&self, snapshot: PeopleSnapshot) -> Result<(), BridgeError>;
}
#[derive(uniffi::Object)]
pub struct PeopleSession {
    store: PeopleStore,
    closed: AtomicBool,
}
impl PeopleSession {
    fn with_client(client: SwapiClient) -> Arc<Self> {
        Arc::new(Self {
            store: PeopleStore::new(client, runtime().handle()),
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
impl PeopleSession {
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, BridgeError> {
        Ok(Self::with_client(
            SwapiClient::new().map_err(|_| BridgeError::Initialization)?,
        ))
    }
    #[uniffi::constructor]
    pub fn with_endpoint(base_url: String) -> Result<Arc<Self>, BridgeError> {
        Ok(Self::with_client(
            SwapiClient::with_endpoint(&base_url, std::time::Duration::from_secs(15))
                .map_err(|_| BridgeError::Initialization)?,
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
        listener: Arc<dyn PeopleListener>,
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
