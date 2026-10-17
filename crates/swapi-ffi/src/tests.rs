use super::*;
use std::{
    sync::{Mutex, Weak},
    time::Duration,
};
use tokio::sync::mpsc;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

struct Listener {
    sender: mpsc::UnboundedSender<CharacterSnapshot>,
    session: Mutex<Weak<CharacterSession>>,
    fail: bool,
}
impl CharacterListener for Listener {
    fn on_state(&self, snapshot: CharacterSnapshot) -> Result<(), BridgeError> {
        if self.fail {
            return Err(BridgeError::Listener);
        }
        if matches!(snapshot.state, CharacterState::Idle)
            && let Some(session) = self.session.lock().unwrap().upgrade()
        {
            session.load(1)?; // Reentrant calls must not deadlock the state actor.
        }
        self.sender.send(snapshot).unwrap();
        Ok(())
    }
}

async fn until(
    rx: &mut mpsc::UnboundedReceiver<CharacterSnapshot>,
    ready: impl Fn(&CharacterState) -> bool,
) -> CharacterSnapshot {
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut previous = None;
        loop {
            let snapshot = rx.recv().await.unwrap();
            if let Some(previous) = previous {
                assert!(snapshot.revision > previous);
            }
            previous = Some(snapshot.revision);
            if ready(&snapshot.state) {
                return snapshot;
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn foreign_boundary_delivers_typed_failure_retry_and_content_without_locks() {
    let server = MockServer::start().await;
    Mock::given(path("/api/people/1/"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(path("/api/people/1/"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../../swapi-core/tests/fixtures/luke.json"),
            "application/json",
        ))
        .with_priority(2)
        .mount(&server)
        .await;
    let session = CharacterSession::with_endpoint(format!("{}/api/", server.uri())).unwrap();
    let (sender, mut rx) = mpsc::unbounded_channel();
    let listener = Arc::new(Listener {
        sender,
        session: Mutex::new(Arc::downgrade(&session)),
        fail: false,
    });
    let weak = Arc::downgrade(&listener);
    let subscription = session.subscribe(listener).unwrap();
    let failure = until(&mut rx, |s| matches!(s, CharacterState::Failed { .. })).await;
    assert_eq!(
        failure.state,
        CharacterState::Failed {
            person_id: 1,
            kind: FailureKind::Http { status: 503 },
            retryable: true
        }
    );
    session.retry().unwrap();
    let ready = until(&mut rx, |s| matches!(s, CharacterState::Ready { .. })).await;
    let CharacterState::Ready { character } = ready.state else {
        panic!("expected character")
    };
    assert_eq!(character.name, "Luke Skywalker");
    assert_eq!(character.height_cm, Measurement::Known { value: 172.0 });
    assert_eq!(character.film_count, 4);
    subscription.cancel();
    // Closure is evidence the task released its listener, not a timing-based sleep.
    assert!(
        tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .is_none()
    );
    assert!(weak.upgrade().is_none());
    session.shutdown();
    assert!(matches!(session.load(2), Err(BridgeError::Closed)));
    assert!(matches!(session.retry(), Err(BridgeError::Closed)));
}

#[tokio::test]
async fn dropped_subscription_and_callback_error_release_foreign_listener() {
    for fail in [false, true] {
        let session = CharacterSession::new().unwrap();
        let (sender, mut rx) = mpsc::unbounded_channel();
        let listener = Arc::new(Listener {
            sender,
            session: Mutex::new(Weak::new()),
            fail,
        });
        let weak = Arc::downgrade(&listener);
        let subscription = session.subscribe(listener).unwrap();
        if !fail {
            until(&mut rx, |s| matches!(s, CharacterState::Idle)).await;
            drop(subscription);
        }
        assert!(
            tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .unwrap()
                .is_none()
        );
        assert!(weak.upgrade().is_none());
        session.shutdown();
    }
}
