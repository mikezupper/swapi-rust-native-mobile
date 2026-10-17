use serde_json::{Value, json};
use std::time::Duration;
use swapi_core::{ApiError, Measurement, ResourceKind, ResourceRef, SwapiClient};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

fn luke() -> Value {
    serde_json::from_str(include_str!("fixtures/luke.json")).unwrap()
}
fn client(server: &MockServer) -> SwapiClient {
    SwapiClient::with_endpoint(&format!("{}/api/", server.uri()), Duration::from_secs(2)).unwrap()
}

#[tokio::test]
async fn real_transport_maps_identity_measurements_and_links() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/people/1/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(luke()))
        .expect(1)
        .mount(&server)
        .await;
    let person = client(&server).person(1).await.unwrap();
    assert_eq!(person.name, "Luke Skywalker");
    assert_eq!(person.height_cm, Measurement::Known(172.0));
    assert_eq!(person.mass_kg, Measurement::Known(77.0));
    assert_eq!(
        person.homeworld,
        ResourceRef {
            kind: ResourceKind::Planet,
            id: 1
        }
    );
    assert_eq!(person.films.len(), 4);
}

#[tokio::test]
async fn unknown_and_not_applicable_remain_distinct() {
    let server = MockServer::start().await;
    let mut value = luke();
    value["height"] = json!("unknown");
    value["mass"] = json!("n/a");
    Mock::given(path("/api/people/1/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(value))
        .mount(&server)
        .await;
    let person = client(&server).person(1).await.unwrap();
    assert_eq!(person.height_cm, Measurement::Unknown);
    assert_eq!(person.mass_kg, Measurement::NotApplicable);
}

#[tokio::test]
async fn grouped_mass_is_parsed_without_losing_precision() {
    let server = MockServer::start().await;
    let mut value = luke();
    value["mass"] = json!("1,358.5");
    Mock::given(path("/api/people/1/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(value))
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).person(1).await.unwrap().mass_kg,
        Measurement::Known(1358.5)
    );
}

#[tokio::test]
async fn search_is_encoded_and_next_page_is_validated_then_reconstructed() {
    let server = MockServer::start().await;
    let query = "Luke & Leia";
    Mock::given(path("/api/people/")).and(query_param("search", query)).and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"count":2,"next":"https://swapi.dev/api/people/?search=Luke+%26+Leia&page=2","results":[luke()]}))).expect(1).mount(&server).await;
    Mock::given(path("/api/people/"))
        .and(query_param("search", query))
        .and(query_param("page", "2"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"count":2,"next":null,"results":[]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server);
    let first = c.people(query, 1).await.unwrap();
    assert_eq!(first.next_page, Some(2));
    assert_eq!(
        c.people(query, first.next_page.unwrap())
            .await
            .unwrap()
            .next_page,
        None
    );
}

#[tokio::test]
async fn rejects_untrusted_or_inconsistent_pagination() {
    for next in [
        "https://evil.example/api/people/?page=2",
        "https://swapi.dev/api/people/?page=1",
        "https://swapi.dev/api/people/?page=2&search=other",
        "https://swapi.dev/api/people/?page=2&page=3",
    ] {
        let server = MockServer::start().await;
        Mock::given(path("/api/people/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"count":1,"next":next,"results":[luke()]})),
            )
            .mount(&server)
            .await;
        assert_eq!(
            client(&server).people("", 1).await,
            Err(ApiError::InvalidData)
        );
    }
}

#[tokio::test]
async fn rejects_bad_json_identity_measurements_and_relationships() {
    let server = MockServer::start().await;
    for (field, value) in [
        ("url", "https://swapi.dev/api/people/2/"),
        ("homeworld", "https://evil.example/api/planets/1/"),
        ("mass", "NaN"),
        ("mass", "1,,2"),
        ("mass", "-5"),
        ("name", ""),
    ] {
        server.reset().await;
        let mut body = luke();
        body[field] = json!(value);
        Mock::given(path("/api/people/1/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        assert_eq!(
            client(&server).person(1).await,
            Err(ApiError::InvalidData),
            "{field}={value}"
        );
    }
    server.reset().await;
    Mock::given(path("/api/people/1/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{broken"))
        .mount(&server)
        .await;
    assert_eq!(client(&server).person(1).await, Err(ApiError::InvalidData));
}

#[tokio::test]
async fn http_errors_preserve_retry_policy_without_following_redirects() {
    let server = MockServer::start().await;
    for status in [301, 404, 429, 503] {
        server.reset().await;
        Mock::given(path("/api/people/1/"))
            .respond_with(
                ResponseTemplate::new(status).insert_header("Location", "https://evil.example/"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = client(&server).person(1).await.unwrap_err();
        assert_eq!(error, ApiError::Http { code: status });
        assert_eq!(error.is_retryable(), status == 429 || status >= 500);
    }
}

#[tokio::test]
async fn request_timeout_includes_response_waiting() {
    let server = MockServer::start().await;
    Mock::given(path("/api/people/1/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(300))
                .set_body_json(luke()),
        )
        .mount(&server)
        .await;
    let c =
        SwapiClient::with_endpoint(&format!("{}/api/", server.uri()), Duration::from_millis(30))
            .unwrap();
    assert_eq!(c.person(1).await, Err(ApiError::Timeout));
}

#[tokio::test]
async fn excessive_response_size_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(path("/api/people/1/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x".repeat(1024 * 1024 + 1)))
        .mount(&server)
        .await;
    assert_eq!(client(&server).person(1).await, Err(ApiError::InvalidData));
}

#[test]
fn validates_resource_references_and_endpoints() {
    for url in [
        "http://swapi.dev/api/people/1/",
        "https://swapi.dev/api/people/0/",
        "https://swapi.dev/api/people/1/?token=a",
        "https://user@swapi.dev/api/people/1/",
        "https://swapi.dev/api/films/1/#x",
    ] {
        assert!(ResourceRef::parse(url).is_err(), "{url}");
    }
    assert!(
        SwapiClient::with_endpoint("https://evil.example/api/", Duration::from_secs(1)).is_err()
    );
}
