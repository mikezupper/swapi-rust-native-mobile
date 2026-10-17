use super::*;
use tokio::sync::oneshot;

type Completion = oneshot::Sender<Result<Page<Person>, ApiError>>;
fn controlled() -> (
    PeopleStore,
    mpsc::UnboundedReceiver<(String, u32, Completion)>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let store = PeopleStore::start(
        Arc::new(move |query, page, _force| {
            let tx = tx.clone();
            Box::pin(async move {
                let (complete, result) = oneshot::channel::<Result<Page<Person>, ApiError>>();
                tx.send((query, page, complete)).unwrap();
                result.await.unwrap().map(Loaded::live)
            })
        }),
        &Handle::current(),
    );
    (store, rx)
}
fn page(ids: &[u32], next_page: Option<u32>) -> Page<Person> {
    Page {
        total: 82,
        next_page,
        items: ids
            .iter()
            .map(|id| {
                let mut value: serde_json::Value =
                    serde_json::from_str(include_str!("../../tests/fixtures/luke.json")).unwrap();
                value["url"] = format!("https://swapi.dev/api/people/{id}/").into();
                Person::try_from(serde_json::from_value::<crate::model::PersonDto>(value).unwrap())
                    .unwrap()
            })
            .collect(),
    }
}
async fn until(
    states: &mut watch::Receiver<PeopleSnapshot>,
    test: impl Fn(&PeopleSnapshot) -> bool,
) -> PeopleSnapshot {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let value = states.borrow_and_update().clone();
            if test(&value) {
                return value;
            }
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap()
}

#[tokio::test(start_paused = true)]
async fn rapid_queries_debounce_and_inflight_replacement_cannot_publish_stale_results() {
    let (store, mut requests) = controlled();
    let mut states = store.observe();
    store.search(" L ".into());
    until(&mut states, |s| s.query == "L").await;
    tokio::time::advance(Duration::from_millis(200)).await;
    store.search("Luke".into());
    until(&mut states, |s| s.query == "Luke").await;
    tokio::time::advance(Duration::from_millis(299)).await;
    assert!(requests.try_recv().is_err());
    let (query, number, mut stale) = requests.recv().await.unwrap();
    assert_eq!((query.as_str(), number), ("Luke", 1));
    store.search("Leia".into());
    stale.closed().await;
    assert!(stale.send(Ok(page(&[1], None))).is_err());
    let (query, _, complete) = requests.recv().await.unwrap();
    assert_eq!(query, "Leia");
    complete.send(Ok(page(&[5], None))).unwrap();
    let ready = until(&mut states, |s| s.phase == PeoplePhase::Ready).await;
    assert_eq!(ready.items[0].id, 5);
    assert_eq!(ready.query, "Leia");
}

#[tokio::test(start_paused = true)]
async fn paging_retains_content_retries_exact_page_deduplicates_and_ignores_duplicate_commands() {
    let (store, mut requests) = controlled();
    let mut states = store.observe();
    store.refresh();
    requests
        .recv()
        .await
        .unwrap()
        .2
        .send(Ok(page(&[1, 2], Some(2))))
        .unwrap();
    until(&mut states, |s| s.phase == PeoplePhase::Ready).await;
    store.load_more();
    let (_, number, complete) = requests.recv().await.unwrap();
    assert_eq!(number, 2);
    store.load_more();
    store.refresh();
    store.retry();
    // Same query is ignored while pending, and provides a queue-processing yield.
    store.search("".into());
    tokio::task::yield_now().await;
    assert!(requests.try_recv().is_err());
    assert_eq!(states.borrow().items.len(), 2);
    complete.send(Err(ApiError::Http { code: 503 })).unwrap();
    let failed = until(&mut states, |s| {
        matches!(s.phase, PeoplePhase::Failed { .. })
    })
    .await;
    assert_eq!(failed.items.len(), 2);
    store.retry();
    let (_, number, complete) = requests.recv().await.unwrap();
    assert_eq!(number, 2);
    complete.send(Ok(page(&[2, 3], None))).unwrap();
    let ready = until(&mut states, |s| s.phase == PeoplePhase::Ready).await;
    assert_eq!(
        ready.items.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    store.load_more();
    store.refresh();
    let (_, number, complete) = requests.recv().await.unwrap();
    assert_eq!(number, 1);
    assert_eq!(states.borrow().items.len(), 3);
    complete.send(Ok(page(&[5], None))).unwrap();
    assert_eq!(
        until(&mut states, |s| s.phase == PeoplePhase::Ready)
            .await
            .items[0]
            .id,
        5
    );
}

#[tokio::test(start_paused = true)]
async fn empty_results_clear_old_content_and_shutdown_cancels_pending_work() {
    let (store, mut requests) = controlled();
    let mut states = store.observe();
    store.refresh();
    requests
        .recv()
        .await
        .unwrap()
        .2
        .send(Ok(page(&[1], Some(2))))
        .unwrap();
    until(&mut states, |s| s.phase == PeoplePhase::Ready).await;
    store.search("nothing".into());
    let (_, _, complete) = requests.recv().await.unwrap();
    assert!(states.borrow().items.is_empty());
    complete
        .send(Ok(Page {
            total: 0,
            next_page: None,
            items: vec![],
        }))
        .unwrap();
    let ready = until(&mut states, |s| s.phase == PeoplePhase::Ready).await;
    assert!(ready.items.is_empty());
    assert_eq!(ready.total, 0);
    store.refresh();
    let (_, _, mut complete) = requests.recv().await.unwrap();
    store.close();
    complete.closed().await;
    assert!(complete.send(Ok(page(&[1], None))).is_err());
    states.borrow_and_update();
    assert!(states.changed().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn refresh_failure_keeps_content_and_nonretryable_failure_is_not_retried() {
    let (store, mut requests) = controlled();
    let mut states = store.observe();
    store.refresh();
    requests
        .recv()
        .await
        .unwrap()
        .2
        .send(Ok(page(&[1], None)))
        .unwrap();
    until(&mut states, |s| s.phase == PeoplePhase::Ready).await;
    store.refresh();
    requests
        .recv()
        .await
        .unwrap()
        .2
        .send(Err(ApiError::InvalidData))
        .unwrap();
    let failed = until(&mut states, |s| {
        matches!(
            s.phase,
            PeoplePhase::Failed {
                operation: PeopleOperation::Refresh,
                ..
            }
        )
    })
    .await;
    assert_eq!(failed.items.len(), 1);
    store.retry();
    tokio::task::yield_now().await;
    assert!(requests.try_recv().is_err());
    // Explicit refresh remains available after a non-retryable response.
    store.refresh();
    assert_eq!(requests.recv().await.unwrap().1, 1);
}
