use serde_json::{Value, json};
use std::time::Duration;
use swapi_core::{ApiError, CatalogStore, PeoplePhase, ResourceKind, SwapiClient};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{path, query_param},
};

const CATEGORIES: [(ResourceKind, u32, &str); 6] = [
    (ResourceKind::Person, 1, "Luke Skywalker"),
    (ResourceKind::Planet, 1, "Tatooine"),
    (ResourceKind::Starship, 9, "Death Star"),
    (ResourceKind::Vehicle, 4, "Sand Crawler"),
    (ResourceKind::Species, 3, "Wookiee"),
    (ResourceKind::Film, 1, "A New Hope"),
];
fn fixture(kind: ResourceKind) -> Value {
    serde_json::from_str::<Value>(include_str!("fixtures/catalog.json")).unwrap()[kind.path()]
        .clone()
}
fn client(server: &MockServer) -> SwapiClient {
    SwapiClient::with_endpoint(&format!("{}/api/", server.uri()), Duration::from_secs(2)).unwrap()
}
#[tokio::test]
async fn every_collection_supports_encoded_search_paging_and_its_own_detail() {
    for (kind, id, name) in CATEGORIES {
        let server = MockServer::start().await;
        let search = " a&b + stars ";
        let mut next = url::Url::parse(&format!("https://swapi.dev/api/{}/", kind.path())).unwrap();
        next.query_pairs_mut()
            .append_pair("page", "2")
            .append_pair("search", search.trim());
        Mock::given(path(format!("/api/{}/", kind.path())))
            .and(query_param("search", search.trim()))
            .and(query_param("page", "1"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!({"count":2,"next":next.as_str(),"results":[fixture(kind)]}),
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path(format!("/api/{}/", kind.path())))
            .and(query_param("page", "2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"count":2,"next":null,"results":[]})),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path(format!("/api/{}/{id}/", kind.path())))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture(kind)))
            .expect(1)
            .mount(&server)
            .await;
        let c = client(&server);
        let first = c.resources(kind, search, 1).await.unwrap();
        assert_eq!(
            (first.items[0].kind, first.items[0].name.as_str()),
            (kind, name)
        );
        assert!(!first.items[0].facts.is_empty());
        assert_eq!(first.next_page, Some(2));
        assert!(
            c.resources(kind, search, 2)
                .await
                .unwrap()
                .next_page
                .is_none()
        );
        let detail = c.resource(kind, id).await.unwrap();
        assert_eq!(detail, first.items[0]);
        if kind == ResourceKind::Film {
            assert_eq!(detail.facts[0].value, "4");
            assert!(!detail.description.unwrap().contains('\r'));
        }
    }
}
#[tokio::test]
async fn rejects_cross_collection_pages_and_malformed_records_without_following_links() {
    let server = MockServer::start().await;
    for next in [
        "https://swapi.dev/api/starships/?page=2",
        "https://evil.example/api/planets/?page=2",
        "https://swapi.dev/api/planets/?page=2&search=other",
    ] {
        server.reset().await;
        Mock::given(path("/api/planets/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"count":1,"next":next,"results":[fixture(ResourceKind::Planet)]}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            client(&server)
                .resources(ResourceKind::Planet, "", 1)
                .await
                .unwrap_err(),
            ApiError::InvalidData
        );
    }
    for (key, value) in [
        ("url", json!("https://swapi.dev/api/starships/1/")),
        ("url", json!("https://swapi.dev/api/planets/2/")),
        ("climate", json!(false)),
        ("name", json!(" ")),
        ("residents", json!(["https://swapi.dev/api/films/1/"])),
    ] {
        server.reset().await;
        let mut planet = fixture(ResourceKind::Planet);
        planet[key] = value;
        Mock::given(path("/api/planets/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(planet))
            .mount(&server)
            .await;
        assert_eq!(
            client(&server)
                .resource(ResourceKind::Planet, 1)
                .await
                .unwrap_err(),
            ApiError::InvalidData
        );
    }
}
#[tokio::test]
async fn detail_session_retries_the_same_category_and_identity() {
    let server = MockServer::start().await;
    Mock::given(path("/api/starships/9/"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    let store = CatalogStore::catalog(
        client(&server),
        ResourceKind::Starship,
        Some(9),
        &tokio::runtime::Handle::current(),
    );
    let mut states = store.observe();
    store.refresh();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if matches!(states.borrow_and_update().phase, PeoplePhase::Failed { .. }) {
                break;
            }
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    server.reset().await;
    Mock::given(path("/api/starships/9/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture(ResourceKind::Starship)))
        .expect(1)
        .mount(&server)
        .await;
    store.retry();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if states.borrow_and_update().phase == PeoplePhase::Ready {
                break;
            }
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(states.borrow().items[0].name, "Death Star");
    store.close();
}
#[tokio::test]
async fn invalid_ids_and_pages_make_no_network_request() {
    let server = MockServer::start().await;
    for (kind, _, _) in CATEGORIES {
        assert_eq!(
            client(&server).resource(kind, 0).await.unwrap_err(),
            ApiError::InvalidRequest
        );
        assert_eq!(
            client(&server).resources(kind, "", 0).await.unwrap_err(),
            ApiError::InvalidRequest
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
