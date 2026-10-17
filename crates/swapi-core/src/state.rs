//! Serialized character state. Observers receive the latest immutable snapshot;
//! slow observers may skip intermediate revisions, but never receive them reversed.
use crate::{ApiError, CacheInfo, Loaded, Person, SwapiClient};
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::{
    runtime::Handle,
    sync::{mpsc, watch},
    task::AbortHandle,
};

#[derive(Clone, Debug, PartialEq)]
pub enum CharacterState {
    Idle,
    Loading { person_id: u32 },
    Ready { person: Person },
    Failed { person_id: u32, error: ApiError },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterSnapshot {
    pub revision: u64,
    pub state: CharacterState,
    pub cache: CacheInfo,
}

enum Command {
    Load(u32),
    Retry,
    Cancel,
}
type Request = Pin<Box<dyn Future<Output = Result<Loaded<Person>, ApiError>> + Send>>;
type Loader = Arc<dyn Fn(u32, bool) -> Request + Send + Sync>;
type CachedLoader = Arc<dyn Fn(u32) -> Option<Loaded<Person>> + Send + Sync>;

pub struct CharacterStore {
    commands: mpsc::UnboundedSender<Command>,
    states: watch::Receiver<CharacterSnapshot>,
    actor: AbortHandle,
}

impl CharacterStore {
    pub fn new(client: SwapiClient, runtime: &Handle) -> Self {
        let cached = client.clone();
        Self::start_cached(
            Arc::new(move |id, force| {
                let client = client.clone();
                Box::pin(async move { client.person_loaded(id, force).await })
            }),
            Arc::new(move |id| cached.cached_person(id)),
            runtime,
        )
    }

    #[cfg(test)]
    fn start(load: Loader, runtime: &Handle) -> Self {
        Self::start_cached(load, Arc::new(|_| None), runtime)
    }
    fn start_cached(load: Loader, cached: CachedLoader, runtime: &Handle) -> Self {
        let (commands, mut inbox) = mpsc::unbounded_channel();
        let (publisher, states) = watch::channel(CharacterSnapshot {
            revision: 0,
            state: CharacterState::Idle,
            cache: CacheInfo::default(),
        });
        let actor = runtime.spawn(async move {
            let mut snapshot = CharacterSnapshot { revision: 0, state: CharacterState::Idle, cache: CacheInfo::default() };
            let mut pending: Option<(u32, Request)> = None;
            loop {
                let next = tokio::select! {
                    biased;
                    command = inbox.recv() => match command {
                        Some(Command::Load(id)) => {
                            // Dropping the previous future cancels its HTTP request.
                            let force = matches!(&snapshot.state, CharacterState::Ready { person } if person.id == id);
                            pending = Some((id, load(id, force)));
                            snapshot.cache = CacheInfo::default();
                            if let Some(saved) = cached(id) {
                                snapshot.cache = saved.cache;
                                CharacterState::Ready { person: saved.value }
                            } else { CharacterState::Loading { person_id: id } }
                        }
                        Some(Command::Cancel) => {
                            pending = None;
                            snapshot.cache = CacheInfo::default();
                            CharacterState::Idle
                        }
                        Some(Command::Retry) => match &snapshot.state {
                            CharacterState::Failed { person_id, error } if error.is_retryable() => {
                                let id = *person_id;
                                pending = Some((id, load(id, true)));
                                CharacterState::Loading { person_id: id }
                            }
                            _ => continue,
                        },
                        None => break,
                    },
                    result = async { pending.as_mut().expect("guarded request").1.as_mut().await }, if pending.is_some() => {
                        let (person_id, _) = pending.take().expect("completed request");
                        match result {
                            Ok(loaded) => {
                                snapshot.cache = loaded.cache;
                                CharacterState::Ready { person: loaded.value }
                            },
                            Err(error) => CharacterState::Failed { person_id, error },
                        }
                    }
                };
                snapshot.revision += 1;
                snapshot.state = next;
                publisher.send_replace(snapshot.clone());
            }
        }).abort_handle();
        Self {
            commands,
            states,
            actor,
        }
    }

    pub fn observe(&self) -> watch::Receiver<CharacterSnapshot> {
        self.states.clone()
    }
    pub fn load(&self, id: u32) {
        let _ = self.commands.send(Command::Load(id));
    }
    pub fn retry(&self) {
        let _ = self.commands.send(Command::Retry);
    }
    pub fn cancel(&self) {
        let _ = self.commands.send(Command::Cancel);
    }
    pub fn close(&self) {
        self.actor.abort();
    }
}

impl Drop for CharacterStore {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        sync::oneshot,
        time::{Duration, timeout},
    };

    type Completion = oneshot::Sender<Result<Person, ApiError>>;
    fn controlled() -> (CharacterStore, mpsc::UnboundedReceiver<(u32, Completion)>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let store = CharacterStore::start(
            Arc::new(move |id, _force| {
                let tx = tx.clone();
                Box::pin(async move {
                    let (complete, result) = oneshot::channel::<Result<Person, ApiError>>();
                    tx.send((id, complete)).unwrap();
                    result.await.unwrap().map(Loaded::live)
                })
            }),
            &Handle::current(),
        );
        (store, rx)
    }
    async fn next(states: &mut watch::Receiver<CharacterSnapshot>) -> CharacterSnapshot {
        timeout(Duration::from_secs(2), states.changed())
            .await
            .unwrap()
            .unwrap();
        states.borrow_and_update().clone()
    }
    fn person(id: u32) -> Person {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/luke.json")).unwrap();
        value["url"] = format!("https://swapi.dev/api/people/{id}/").into();
        Person::try_from(serde_json::from_value::<crate::model::PersonDto>(value).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn loading_success_and_late_subscriber_have_monotonic_snapshots() {
        let (store, mut requests) = controlled();
        let mut states = store.observe();
        assert_eq!(states.borrow_and_update().revision, 0);
        store.load(1);
        assert_eq!(
            next(&mut states).await.state,
            CharacterState::Loading { person_id: 1 }
        );
        let (id, complete) = requests.recv().await.unwrap();
        assert_eq!(id, 1);
        complete.send(Ok(person(1))).unwrap();
        let ready = next(&mut states).await;
        assert_eq!(
            ready,
            CharacterSnapshot {
                revision: 2,
                state: CharacterState::Ready { person: person(1) },
                cache: CacheInfo::default()
            }
        );
        assert_eq!(*store.observe().borrow(), ready);
    }

    #[tokio::test]
    async fn replacement_and_cancel_drop_requests_before_they_can_publish() {
        let (store, mut requests) = controlled();
        let mut states = store.observe();
        store.load(1);
        let (_, mut old) = requests.recv().await.unwrap();
        store.load(2);
        let (id, mut current) = requests.recv().await.unwrap();
        assert_eq!(id, 2);
        timeout(Duration::from_secs(2), old.closed()).await.unwrap();
        assert!(old.send(Ok(person(1))).is_err());
        store.cancel();
        timeout(Duration::from_secs(2), current.closed())
            .await
            .unwrap();
        assert!(current.send(Ok(person(2))).is_err());
        assert_eq!(next(&mut states).await.state, CharacterState::Idle);
    }

    #[tokio::test]
    async fn retry_uses_failed_identity_and_ignores_non_retryable_errors() {
        let (store, mut requests) = controlled();
        let mut states = store.observe();
        store.load(2);
        requests
            .recv()
            .await
            .unwrap()
            .1
            .send(Err(ApiError::Network))
            .unwrap();
        while !matches!(next(&mut states).await.state, CharacterState::Failed { .. }) {}
        store.retry();
        let (id, complete) = requests.recv().await.unwrap();
        assert_eq!(id, 2);
        complete.send(Err(ApiError::InvalidData)).unwrap();
        while !matches!(
            next(&mut states).await.state,
            CharacterState::Failed {
                error: ApiError::InvalidData,
                ..
            }
        ) {}
        store.retry();
        // A subsequent cancel acts as a command barrier; retry must not invoke the loader.
        store.cancel();
        while next(&mut states).await.state != CharacterState::Idle {}
        assert!(requests.try_recv().is_err());
    }

    #[tokio::test]
    async fn close_releases_request_and_observers() {
        let (store, mut requests) = controlled();
        let mut states = store.observe();
        store.load(1);
        let (_, mut complete) = requests.recv().await.unwrap();
        states.borrow_and_update();
        store.close();
        timeout(Duration::from_secs(2), complete.closed())
            .await
            .unwrap();
        assert!(
            timeout(Duration::from_secs(2), states.changed())
                .await
                .unwrap()
                .is_err()
        );
        store.close();
    }
}
