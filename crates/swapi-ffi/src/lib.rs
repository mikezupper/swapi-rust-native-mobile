//! Small, synchronous command boundary. HTTP and notifications run on Rust's
//! process-lifetime runtime; native adapters marshal snapshots onto their UI thread.
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use swapi_core::{ApiError, CharacterStore, SwapiClient};
use tokio::{runtime::Runtime, task::AbortHandle};

uniffi::setup_scaffolding!();

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("holocron-core")
            .enable_all()
            .build()
            .expect("create shared Rust runtime")
    })
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum Measurement {
    Known { value: f64 },
    Unknown,
    NotApplicable,
}
impl From<swapi_core::Measurement> for Measurement {
    fn from(value: swapi_core::Measurement) -> Self {
        match value {
            swapi_core::Measurement::Known(value) => Self::Known { value },
            swapi_core::Measurement::Unknown => Self::Unknown,
            swapi_core::Measurement::NotApplicable => Self::NotApplicable,
        }
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct Character {
    pub id: u32,
    pub name: String,
    pub height_cm: Measurement,
    pub mass_kg: Measurement,
    pub birth_year: String,
    pub eye_color: String,
    pub hair_color: String,
    pub film_count: u32,
}
impl From<swapi_core::Person> for Character {
    fn from(person: swapi_core::Person) -> Self {
        Self {
            id: person.id,
            name: person.name,
            height_cm: person.height_cm.into(),
            mass_kg: person.mass_kg.into(),
            birth_year: person.birth_year,
            eye_color: person.eye_color,
            hair_color: person.hair_color,
            film_count: person.films.len() as u32,
        }
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum FailureKind {
    Network,
    Timeout,
    Http { status: u16 },
    InvalidData,
    InvalidRequest,
    Storage,
}
impl From<ApiError> for FailureKind {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::Network => Self::Network,
            ApiError::Timeout => Self::Timeout,
            ApiError::Http { code } => Self::Http { status: code },
            ApiError::InvalidData => Self::InvalidData,
            ApiError::InvalidRequest => Self::InvalidRequest,
            ApiError::Storage => Self::Storage,
        }
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum CharacterState {
    Idle,
    Loading {
        person_id: u32,
    },
    Ready {
        character: Character,
    },
    Failed {
        person_id: u32,
        kind: FailureKind,
        retryable: bool,
    },
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CharacterSnapshot {
    pub revision: u64,
    pub state: CharacterState,
    pub cache: CacheStatus,
}

impl From<swapi_core::CharacterSnapshot> for CharacterSnapshot {
    fn from(snapshot: swapi_core::CharacterSnapshot) -> Self {
        use swapi_core::CharacterState as Core;
        let state = match snapshot.state {
            Core::Idle => CharacterState::Idle,
            Core::Loading { person_id } => CharacterState::Loading { person_id },
            Core::Ready { person } => CharacterState::Ready {
                character: person.into(),
            },
            Core::Failed { person_id, error } => CharacterState::Failed {
                person_id,
                retryable: error.is_retryable(),
                kind: error.into(),
            },
        };
        Self {
            revision: snapshot.revision,
            state,
            cache: snapshot.cache.into(),
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum BridgeError {
    #[error("could not initialize the HTTP client")]
    Initialization,
    #[error("session has been closed")]
    Closed,
    #[error("state listener failed")]
    Listener,
    #[error("could not open the local archive")]
    Storage,
}

impl From<uniffi::UnexpectedUniFFICallbackError> for BridgeError {
    fn from(_: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Listener
    }
}

#[uniffi::export(with_foreign)]
pub trait CharacterListener: Send + Sync {
    fn on_state(&self, snapshot: CharacterSnapshot) -> Result<(), BridgeError>;
}

#[derive(uniffi::Object)]
pub struct CharacterSession {
    store: CharacterStore,
    closed: AtomicBool,
}

impl CharacterSession {
    fn with_client(client: SwapiClient) -> Arc<Self> {
        Arc::new(Self {
            store: CharacterStore::new(client, runtime().handle()),
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
impl CharacterSession {
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, BridgeError> {
        Ok(Self::with_client(
            SwapiClient::new().map_err(|_| BridgeError::Initialization)?,
        ))
    }
    /// Deterministic integration tests may serve fixtures over loopback HTTP.
    /// The core rejects arbitrary remote endpoints and malformed URLs.
    #[uniffi::constructor]
    pub fn with_endpoint(base_url: String) -> Result<Arc<Self>, BridgeError> {
        Ok(Self::with_client(
            SwapiClient::with_endpoint(&base_url, std::time::Duration::from_secs(15))
                .map_err(|_| BridgeError::Initialization)?,
        ))
    }
    pub fn load(&self, person_id: u32) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.load(person_id);
        Ok(())
    }
    pub fn retry(&self) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.retry();
        Ok(())
    }
    pub fn cancel(&self) -> Result<(), BridgeError> {
        self.ensure_open()?;
        self.store.cancel();
        Ok(())
    }
    pub fn subscribe(
        &self,
        listener: Arc<dyn CharacterListener>,
    ) -> Result<Arc<Subscription>, BridgeError> {
        self.ensure_open()?;
        let mut states = self.store.observe();
        let cancelled = Arc::new(AtomicBool::new(false));
        let guard = cancelled.clone();
        let task = runtime()
            .spawn(async move {
                loop {
                    // End the watch borrow before calling foreign code: no internal lock
                    // may be held during a listener callback (which may reenter Rust).
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
    pub fn shutdown(&self) {
        self.closed.store(true, Ordering::Release);
        self.store.close();
    }
}

#[derive(uniffi::Object)]
pub struct Subscription {
    cancelled: Arc<AtomicBool>,
    task: AbortHandle,
}

#[uniffi::export]
impl Subscription {
    /// Stops future callbacks. A callback already running may finish; the native
    /// adapter must also invalidate queued UI delivery before calling this.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.task.abort();
    }
}
impl Drop for Subscription {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests;

mod people;
pub use people::*;

mod catalog;
pub use catalog::*;
mod archive;
pub use archive::*;
