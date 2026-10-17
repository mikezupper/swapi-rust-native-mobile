use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use swapi_core::{
    ArchiveCache, ArchiveSnapshot, ArchiveStore, CatalogStore, DownloadState, PeoplePhase,
    ResourceKind, SwapiClient,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{path, query_param},
};

fn planet(id: u32) -> Value {
    let mut value =
        serde_json::from_str::<Value>(include_str!("fixtures/catalog.json")).unwrap()["planets"]
            .clone();
    value["url"] = json!(format!("https://swapi.dev/api/planets/{id}/"));
    value["name"] = json!(format!("Planet {id}"));
    value
}
fn client(server: &MockServer, db: &std::path::Path) -> (SwapiClient, Arc<ArchiveCache>) {
    let client =
        SwapiClient::with_endpoint(&format!("{}/api/", server.uri()), Duration::from_secs(3))
            .unwrap();
    let cache = Arc::new(ArchiveCache::open(db, client.endpoint()).unwrap());
    (client.with_cache(cache.clone()), cache)
}
async fn page(server: &MockServer, number: u32, next: bool) {
    Mock::given(path("/api/planets/")).and(query_param("page", number.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"count":2,"next":if next { Some("https://swapi.dev/api/planets/?page=2") } else {None},"results":[planet(number)]})))
        .mount(server).await;
}
async fn until(
    states: &mut tokio::sync::watch::Receiver<ArchiveSnapshot>,
    predicate: impl Fn(&ArchiveSnapshot) -> bool,
) -> ArchiveSnapshot {
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            let value = states.borrow_and_update().clone();
            if predicate(&value) {
                return value;
            }
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn restart_offline_search_and_clear_keep_favorites_and_detail() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("archive.sqlite");
    page(&server, 1, true).await;
    page(&server, 2, false).await;
    {
        let (c, cache) = client(&server, &db);
        c.resources(ResourceKind::Planet, "", 1).await.unwrap();
        c.resources(ResourceKind::Planet, "", 2).await.unwrap();
        cache.set_favorite(ResourceKind::Planet, 2, true).unwrap();
        cache.set_offline(true).unwrap();
    }
    server.reset().await;
    let (c, cache) = client(&server, &db);
    assert!(cache.is_offline());
    assert_eq!(
        c.resource(ResourceKind::Planet, 2).await.unwrap().name,
        "Planet 2"
    );
    assert_eq!(cache.records("PLANET 2", false).unwrap().len(), 1);
    assert_eq!(cache.records("", true).unwrap()[0].record.id, 2);
    assert!(c.resource(ResourceKind::Starship, 2).await.is_err());
    assert!(
        c.resources(ResourceKind::Planet, "missing search", 1)
            .await
            .is_err()
    );
    cache.clear_cache().unwrap();
    assert_eq!(cache.records("", false).unwrap().len(), 1);
    assert_eq!(c.resource(ResourceKind::Planet, 2).await.unwrap().id, 2);
    assert!(c.resource(ResourceKind::Planet, 1).await.is_err());
    assert!(cache.collections().unwrap().iter().all(|c| !c.complete));
    assert!(server.received_requests().await.unwrap().is_empty());
}
#[tokio::test]
async fn fresh_cache_avoids_http_and_namespaces_do_not_share_data() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("archive.sqlite");
    let (c, _) = client(&server, &db);
    page(&server, 1, true).await;
    c.resources(ResourceKind::Planet, "", 1).await.unwrap();
    c.resources(ResourceKind::Planet, "", 1).await.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    let isolated = ArchiveCache::open(&db, "https://swapi.dev/api/").unwrap();
    assert!(isolated.records("", false).unwrap().is_empty());
}
#[tokio::test]
async fn stale_records_publish_before_refresh_and_bad_refresh_preserves_data() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("archive.sqlite");
    let (c, _) = client(&server, &db);
    page(&server, 1, true).await;
    c.resources(ResourceKind::Planet, "", 1).await.unwrap();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute("UPDATE responses SET saved_at=1", [])
        .unwrap();
    server.reset().await;
    Mock::given(path("/api/planets/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(150))
                .set_body_json(json!({"count":1,"next":null,"results":[{"name":"bad"}]})),
        )
        .mount(&server)
        .await;
    let store = CatalogStore::catalog(
        c,
        ResourceKind::Planet,
        None,
        &tokio::runtime::Handle::current(),
    );
    let mut states = store.observe();
    store.refresh();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if states.borrow().phase == PeoplePhase::Refreshing {
                break;
            }
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(states.borrow().items[0].name, "Planet 1");
    assert_eq!(states.borrow().cache.saved_at, Some(1));
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if states.borrow().phase == PeoplePhase::Ready {
                break;
            }
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(states.borrow().cache.refresh_failed);
    assert_eq!(states.borrow().items[0].name, "Planet 1");
    store.close();
}
#[tokio::test]
async fn download_failure_retry_and_restart_report_complete_only_after_all_pages() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("archive.sqlite");
    let (c, cache) = client(&server, &db);
    page(&server, 1, true).await;
    Mock::given(path("/api/planets/"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let store = ArchiveStore::new(c, cache.clone(), &tokio::runtime::Handle::current());
    let mut states = store.observe();
    store.download(ResourceKind::Planet);
    let failed = until(&mut states, |s| {
        matches!(s.download, DownloadState::Failed { .. })
    })
    .await;
    assert_eq!(failed.items.len(), 1);
    assert!(
        !failed
            .collections
            .iter()
            .find(|c| c.kind == ResourceKind::Planet)
            .unwrap()
            .complete
    );
    server.reset().await;
    page(&server, 2, false).await;
    store.download(ResourceKind::Planet);
    let done = until(&mut states, |s| {
        matches!(s.download, DownloadState::Complete { count: 2, .. })
    })
    .await;
    assert!(
        done.collections
            .iter()
            .find(|c| c.kind == ResourceKind::Planet)
            .unwrap()
            .complete
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1); // page 1 reused
    store.close();
    drop(store);
    drop(cache);
    let (c, cache) = client(&server, &db);
    cache.set_offline(true).unwrap();
    assert!(
        cache
            .collections()
            .unwrap()
            .iter()
            .find(|c| c.kind == ResourceKind::Planet)
            .unwrap()
            .complete
    );
    assert_eq!(
        c.resource(ResourceKind::Planet, 2).await.unwrap().name,
        "Planet 2"
    );
}
#[tokio::test]
async fn offline_switch_cancels_inflight_http_without_saving_its_late_response() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let (c, cache) = client(&server, &dir.path().join("archive.sqlite"));
    Mock::given(path("/api/planets/1/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(2))
                .set_body_json(planet(1)),
        )
        .mount(&server)
        .await;
    let request = tokio::spawn(async move { c.resource(ResourceKind::Planet, 1).await });
    tokio::time::timeout(Duration::from_secs(1), async {
        while server.received_requests().await.unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    cache.set_offline(true).unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(300), request)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert!(cache.records("", false).unwrap().is_empty());
}
#[tokio::test]
async fn cancelling_download_preserves_partial_records_without_completeness() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let (c, cache) = client(&server, &dir.path().join("archive.sqlite"));
    page(&server, 1, true).await;
    Mock::given(path("/api/planets/"))
        .and(query_param("page", "2"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(2))
                .set_body_json(json!({"count":2,"next":null,"results":[planet(2)]})),
        )
        .mount(&server)
        .await;
    let store = ArchiveStore::new(c, cache, &tokio::runtime::Handle::current());
    let mut states = store.observe();
    store.download(ResourceKind::Planet);
    until(&mut states, |s| {
        matches!(s.download, DownloadState::Running { saved: 1, .. })
    })
    .await;
    store.cancel_download();
    let stopped = until(&mut states, |s| s.download == DownloadState::Idle).await;
    assert_eq!(stopped.items.len(), 1);
    assert!(stopped.collections.iter().all(|c| !c.complete));
    store.close();
}

#[tokio::test]
async fn explicit_refresh_revalidates_a_fresh_response_and_preserves_favorite() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let (c, cache) = client(&server, &dir.path().join("archive.sqlite"));
    page(&server, 1, true).await;
    c.resources(ResourceKind::Planet, "", 1).await.unwrap();
    cache.set_favorite(ResourceKind::Planet, 1, true).unwrap();
    server.reset().await;
    let mut updated = planet(1);
    updated["name"] = json!("Updated planet");
    Mock::given(path("/api/planets/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"count":1,"next":null,"results":[updated]})),
        )
        .mount(&server)
        .await;
    let store = CatalogStore::catalog(
        c,
        ResourceKind::Planet,
        None,
        &tokio::runtime::Handle::current(),
    );
    let mut states = store.observe();
    store.refresh();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if states.borrow().phase == PeoplePhase::Ready {
                break;
            }
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(server.received_requests().await.unwrap().is_empty());
    store.refresh();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if states
                .borrow()
                .items
                .first()
                .is_some_and(|r| r.name == "Updated planet")
            {
                break;
            }
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert!(!states.borrow().cache.from_cache);
    assert_eq!(
        cache.records("", true).unwrap()[0].record.name,
        "Updated planet"
    );
    store.close();
}

#[test]
fn newer_schema_is_rejected_without_modifying_existing_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("archive.sqlite");
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection.execute_batch("PRAGMA user_version=99; CREATE TABLE future_data(value TEXT); INSERT INTO future_data VALUES('preserve me');").unwrap();
    assert!(matches!(
        ArchiveCache::open(&db, "https://swapi.dev/api/"),
        Err(swapi_core::ApiError::Storage)
    ));
    let value: String = connection
        .query_row("SELECT value FROM future_data", [], |r| r.get(0))
        .unwrap();
    assert_eq!(value, "preserve me");
}

#[tokio::test]
async fn a_new_collection_response_replaces_an_older_cached_detail() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let (c, cache) = client(&server, &dir.path().join("archive.sqlite"));
    Mock::given(path("/api/planets/1/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(planet(1)))
        .mount(&server)
        .await;
    assert_eq!(
        c.resource(ResourceKind::Planet, 1).await.unwrap().name,
        "Planet 1"
    );
    let mut updated = planet(1);
    updated["name"] = json!("New name");
    Mock::given(path("/api/planets/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"count":1,"next":null,"results":[updated]})),
        )
        .mount(&server)
        .await;
    c.resources(ResourceKind::Planet, "", 1).await.unwrap();
    cache.set_offline(true).unwrap();
    assert_eq!(
        c.resource(ResourceKind::Planet, 1).await.unwrap().name,
        "New name"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}
