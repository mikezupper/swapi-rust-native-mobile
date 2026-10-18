use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use swapi_core::{
    ArchiveCache, CatalogStore, PeoplePhase, ResourceKind, SwapiClient, compare_ships,
    parse_record_link,
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

async fn setup() -> (
    MockServer,
    tempfile::TempDir,
    SwapiClient,
    Arc<ArchiveCache>,
) {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let client =
        SwapiClient::with_endpoint(&format!("{}/api/", server.uri()), Duration::from_secs(2))
            .unwrap();
    let cache =
        Arc::new(ArchiveCache::open(dir.path().join("archive.sqlite"), client.endpoint()).unwrap());
    (server, dir, client.with_cache(cache.clone()), cache)
}
async fn mount(server: &MockServer, kind: &str, id: u32, changes: Value) {
    let mut value =
        serde_json::from_str::<Value>(include_str!("fixtures/catalog.json")).unwrap()[kind].clone();
    value["url"] = json!(format!("https://swapi.dev/api/{kind}/{id}/"));
    for (key, v) in changes.as_object().unwrap() {
        value[key] = v.clone();
    }
    Mock::given(path(format!("/api/{kind}/{id}/")))
        .respond_with(ResponseTemplate::new(200).set_body_json(value))
        .mount(server)
        .await;
}
#[tokio::test]
async fn relationships_resolve_from_saved_records_without_http() {
    let (server, _dir, client, cache) = setup().await;
    mount(
        &server,
        "people",
        1,
        json!({"starships":["https://swapi.dev/api/starships/9/"]}),
    )
    .await;
    mount(&server, "planets", 1, json!({})).await;
    let person = client.resource(ResourceKind::Person, 1).await.unwrap();
    assert!(
        person
            .related
            .iter()
            .any(|g| g.kind == ResourceKind::Starship && g.ids == [9])
    );
    client.resource(ResourceKind::Planet, 1).await.unwrap();
    cache.set_offline(true).unwrap();
    server.reset().await;
    let store = CatalogStore::related(
        client,
        ResourceKind::Planet,
        vec![1, 1],
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
    assert_eq!(states.borrow().items.len(), 1);
    assert_eq!(states.borrow().items[0].name, "Tatooine");
    assert!(states.borrow().cache.offline);
    assert!(server.received_requests().await.unwrap().is_empty());
}
#[tokio::test]
async fn discovery_recency_comparison_and_favorites_survive_restart() {
    let (server, dir, client, cache) = setup().await;
    for id in 1..=3 {
        mount(
            &server,
            "starships",
            id,
            json!({"name":format!("Ship {id}")}),
        )
        .await;
        client.resource(ResourceKind::Starship, id).await.unwrap();
    }
    mount(&server, "planets", 1, json!({})).await;
    client.resource(ResourceKind::Planet, 1).await.unwrap();
    cache.visit(ResourceKind::Starship, 1).unwrap();
    cache.visit(ResourceKind::Planet, 1).unwrap();
    cache.visit(ResourceKind::Starship, 1).unwrap();
    cache.select_ship(1, true).unwrap();
    cache.select_ship(2, true).unwrap();
    assert!(cache.select_ship(3, true).is_err());
    cache.set_favorite(ResourceKind::Starship, 1, true).unwrap();
    let reopened =
        ArchiveCache::open(dir.path().join("archive.sqlite"), client.endpoint()).unwrap();
    let discovery = reopened.exploration(5).unwrap();
    assert_eq!(discovery.recent.len(), 2);
    assert_eq!(discovery.recent[0].record.name, "Ship 1");
    assert_eq!(discovery.featured.unwrap().record.name, "Tatooine");
    assert_eq!(discovery.widget[0].record.name, "Ship 1");
    assert_eq!(discovery.ships.len(), 2);
    assert_eq!(discovery.metrics.len(), 4);
    reopened.select_ship(1, false).unwrap();
    reopened.select_ship(3, true).unwrap();
    assert_eq!(
        reopened
            .exploration(5)
            .unwrap()
            .ships
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        [2, 3]
    );
    reopened.clear_cache().unwrap();
    assert!(reopened.exploration(5).unwrap().ships.is_empty());
    assert_eq!(reopened.exploration(5).unwrap().recent.len(), 1);
}
#[tokio::test]
async fn comparison_preserves_ranges_and_unknowns_without_misleading_bars() {
    let (server, _dir, client, _) = setup().await;
    mount(&server,"starships",1,json!({"length":"1,200", "crew":"30-165", "cargo_capacity":"unknown", "hyperdrive_rating":"n/a"})).await;
    mount(
        &server,
        "starships",
        2,
        json!({"length":"600","crew":"3","cargo_capacity":"0","hyperdrive_rating":"0.5"}),
    )
    .await;
    let metrics = compare_ships(&[
        client.resource(ResourceKind::Starship, 1).await.unwrap(),
        client.resource(ResourceKind::Starship, 2).await.unwrap(),
    ]);
    assert_eq!(metrics[0].values[1].fraction, Some(0.5));
    assert_eq!(metrics[1].values[0].fraction, None);
    assert_eq!(metrics[1].values[0].display, "30-165");
    assert_eq!(metrics[2].values[0].display, "Unknown");
    assert_eq!(metrics[2].values[1].fraction, Some(0.0));
    assert_eq!(metrics[3].values[0].display, "Not applicable");
    assert!(metrics[3].lower_is_better);
}
#[tokio::test]
async fn malformed_relationships_are_rejected_before_caching() {
    let (server, _dir, client, cache) = setup().await;
    mount(
        &server,
        "people",
        1,
        json!({"starships":["https://evil.example/api/starships/1/"]}),
    )
    .await;
    assert!(client.resource(ResourceKind::Person, 1).await.is_err());
    assert!(cache.records("", false).unwrap().is_empty());
}
#[test]
fn deep_links_only_accept_known_record_routes() {
    assert_eq!(
        parse_record_link("holocron://record/planets/1").unwrap().id,
        1
    );
    for bad in [
        "https://record/planets/1",
        "holocron://record/planets/0",
        "holocron://record/planets/1?url=evil",
        "holocron://record/files/1",
        "holocron://record/people/1/more",
        "holocron://user@record/people/1",
    ] {
        assert!(parse_record_link(bad).is_err(), "{bad}");
    }
}

#[tokio::test]
async fn relationship_groups_page_without_duplicate_records_and_cancel_on_close() {
    let (server, _dir, client, _) = setup().await;
    for id in 1..=7 {
        mount(
            &server,
            "planets",
            id,
            json!({"name":format!("World {id}")}),
        )
        .await;
    }
    let store = CatalogStore::related(
        client,
        ResourceKind::Planet,
        vec![1, 2, 3, 4, 5, 6, 7, 1],
        &tokio::runtime::Handle::current(),
    );
    let mut states = store.observe();
    store.refresh();
    async fn ready(
        states: &mut tokio::sync::watch::Receiver<swapi_core::CatalogSnapshot>,
        count: usize,
    ) {
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if states.borrow().phase == PeoplePhase::Ready
                    && states.borrow().items.len() == count
                {
                    break;
                }
                states.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
    }
    ready(&mut states, 6).await;
    assert_eq!(states.borrow().next_page, Some(2));
    assert_eq!(states.borrow().total, 7);
    store.load_more();
    ready(&mut states, 7).await;
    assert_eq!(states.borrow().items[6].name, "World 7");
    assert_eq!(states.borrow().next_page, None);
    assert_eq!(server.received_requests().await.unwrap().len(), 7);
    store.close();
    // The actor's publisher is released on shutdown; listeners cannot be retained.
    while states.changed().await.is_ok() {}
}

#[tokio::test]
async fn schema_one_migration_preserves_existing_favorites() {
    let (server, dir, client, cache) = setup().await;
    mount(&server, "planets", 1, json!({})).await;
    client.resource(ResourceKind::Planet, 1).await.unwrap();
    cache.set_favorite(ResourceKind::Planet, 1, true).unwrap();
    cache.set_offline(true).unwrap();
    let endpoint = client.endpoint().to_string();
    drop(client);
    drop(cache);
    let path = dir.path().join("archive.sqlite");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TABLE visits; DROP TABLE comparison; PRAGMA user_version=1;")
        .unwrap();
    drop(connection);
    let migrated = ArchiveCache::open(&path, &endpoint).unwrap();
    assert!(migrated.is_offline());
    assert_eq!(
        migrated.records("", true).unwrap()[0].record.name,
        "Tatooine"
    );
    migrated.visit(ResourceKind::Planet, 1).unwrap();
    assert_eq!(migrated.exploration(1).unwrap().recent.len(), 1);
}
