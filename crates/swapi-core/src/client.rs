use crate::{ApiError, PeoplePage, Person, model::PersonDto};
use serde::{Deserialize, de::DeserializeOwned};
use std::time::Duration;
use url::Url;

const PRODUCTION: &str = "https://swapi.dev/api/";
const MAX_BODY: usize = 1024 * 1024;

#[derive(Clone)]
pub struct SwapiClient {
    http: reqwest::Client,
    base: Url,
}

impl SwapiClient {
    pub fn new() -> Result<Self, ApiError> {
        Self::with_endpoint(PRODUCTION, Duration::from_secs(15))
    }

    /// A loopback HTTP endpoint can replace production for deterministic tests.
    pub fn with_endpoint(base: &str, timeout: Duration) -> Result<Self, ApiError> {
        let base = Url::parse(base).map_err(|_| ApiError::InvalidRequest)?;
        let loopback = matches!(base.host_str(), Some("127.0.0.1" | "[::1]" | "localhost"));
        if base.as_str() != PRODUCTION
            && !(loopback && base.scheme() == "http" && base.path() == "/api/")
        {
            return Err(ApiError::InvalidRequest);
        }
        if !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || timeout.is_zero()
        {
            return Err(ApiError::InvalidRequest);
        }
        // A bundled Mozilla trust store works identically through both FFI targets;
        // no platform-verifier JVM initialization is required on Android.
        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let tls = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| ApiError::Network)?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .tls_backend_preconfigured(tls)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(|_| ApiError::Network)?;
        Ok(Self { http, base })
    }

    pub async fn person(&self, id: u32) -> Result<Person, ApiError> {
        if id == 0 {
            return Err(ApiError::InvalidRequest);
        }
        let url = self
            .base
            .join(&format!("people/{id}/"))
            .map_err(|_| ApiError::InvalidRequest)?;
        let dto: PersonDto = self.get(url).await?;
        let person = Person::try_from(dto)?;
        if person.id != id {
            return Err(ApiError::InvalidData);
        }
        Ok(person)
    }

    pub async fn people(&self, search: &str, page: u32) -> Result<PeoplePage, ApiError> {
        if page == 0 {
            return Err(ApiError::InvalidRequest);
        }
        let search = search.trim();
        let mut url = self
            .base
            .join("people/")
            .map_err(|_| ApiError::InvalidRequest)?;
        url.query_pairs_mut().append_pair("page", &page.to_string());
        if !search.is_empty() {
            url.query_pairs_mut().append_pair("search", search);
        }
        let dto: PageDto = self.get(url).await?;
        let next_page = dto
            .next
            .as_deref()
            .map(|s| self.next_page(s, search, page))
            .transpose()?;
        let people = dto
            .results
            .into_iter()
            .map(Person::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if people.len() > dto.count as usize {
            return Err(ApiError::InvalidData);
        }
        Ok(PeoplePage {
            total: dto.count,
            next_page,
            people,
        })
    }

    fn next_page(&self, value: &str, search: &str, current: u32) -> Result<u32, ApiError> {
        let url = Url::parse(value).map_err(|_| ApiError::InvalidData)?;
        let production = Url::parse(PRODUCTION).expect("constant URL");
        if (url.origin() != self.base.origin() && url.origin() != production.origin())
            || url.path() != "/api/people/"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err(ApiError::InvalidData);
        }
        let mut page = None;
        let mut returned_search = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "page" if page.is_none() => {
                    page = Some(value.parse::<u32>().map_err(|_| ApiError::InvalidData)?)
                }
                "search" if returned_search.is_none() => returned_search = Some(value.into_owned()),
                _ => return Err(ApiError::InvalidData),
            }
        }
        let page = page
            .filter(|&p| Some(p) == current.checked_add(1))
            .ok_or(ApiError::InvalidData)?;
        if returned_search.as_deref().unwrap_or("") != search {
            return Err(ApiError::InvalidData);
        }
        Ok(page)
    }

    async fn get<T: DeserializeOwned>(&self, url: Url) -> Result<T, ApiError> {
        let mut response = self.http.get(url).send().await.map_err(network_error)?;
        if !response.status().is_success() {
            return Err(ApiError::Http {
                code: response.status().as_u16(),
            });
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BODY as u64)
        {
            return Err(ApiError::InvalidData);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(network_error)? {
            if body.len() + chunk.len() > MAX_BODY {
                return Err(ApiError::InvalidData);
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body).map_err(|_| ApiError::InvalidData)
    }
}

fn network_error(error: reqwest::Error) -> ApiError {
    if error.is_timeout() {
        ApiError::Timeout
    } else {
        ApiError::Network
    }
}

#[derive(Deserialize)]
struct PageDto {
    count: u32,
    next: Option<String>,
    results: Vec<PersonDto>,
}
