use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ApiError {
    #[error("request timed out")]
    Timeout,
    #[error("network unavailable")]
    Network,
    #[error("HTTP status {code}")]
    Http { code: u16 },
    #[error("invalid API response")]
    InvalidData,
    #[error("invalid request")]
    InvalidRequest,
}

impl ApiError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout
                | Self::Network
                | Self::Http {
                    code: 429 | 500..=599
                }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Measurement {
    Known(f64),
    Unknown,
    NotApplicable,
}

impl Measurement {
    pub(crate) fn parse(value: &str) -> Result<Self, ApiError> {
        match value {
            "unknown" => return Ok(Self::Unknown),
            "n/a" => return Ok(Self::NotApplicable),
            _ => {}
        }
        let parts: Vec<_> = value.split('.').collect();
        if parts.len() > 2 || parts.iter().any(|part| part.is_empty()) {
            return Err(ApiError::InvalidData);
        }
        let groups: Vec<_> = parts[0].split(',').collect();
        if groups.len() > 1
            && (groups[0].is_empty()
                || groups[0].len() > 3
                || groups[1..].iter().any(|g| g.len() != 3))
        {
            return Err(ApiError::InvalidData);
        }
        if !groups
            .iter()
            .all(|g| !g.is_empty() && g.bytes().all(|b| b.is_ascii_digit()))
            || parts
                .get(1)
                .is_some_and(|p| !p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(ApiError::InvalidData);
        }
        let number: f64 = value
            .replace(',', "")
            .parse()
            .map_err(|_| ApiError::InvalidData)?;
        if !number.is_finite() {
            return Err(ApiError::InvalidData);
        }
        Ok(Self::Known(number))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Person,
    Planet,
    Film,
    Species,
    Starship,
    Vehicle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRef {
    pub kind: ResourceKind,
    pub id: u32,
}

impl ResourceRef {
    pub fn parse(value: &str) -> Result<Self, ApiError> {
        let url = Url::parse(value).map_err(|_| ApiError::InvalidData)?;
        if url.scheme() != "https"
            || url.host_str() != Some("swapi.dev")
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(ApiError::InvalidData);
        }
        let parts: Vec<_> = url.path().split('/').collect();
        if parts.len() != 5 || !parts[0].is_empty() || parts[1] != "api" || !parts[4].is_empty() {
            return Err(ApiError::InvalidData);
        }
        let kind = match parts[2] {
            "people" => ResourceKind::Person,
            "planets" => ResourceKind::Planet,
            "films" => ResourceKind::Film,
            "species" => ResourceKind::Species,
            "starships" => ResourceKind::Starship,
            "vehicles" => ResourceKind::Vehicle,
            _ => return Err(ApiError::InvalidData),
        };
        let id = parts[3].parse().map_err(|_| ApiError::InvalidData)?;
        if id == 0 {
            return Err(ApiError::InvalidData);
        }
        Ok(Self { kind, id })
    }

    fn of_kind(value: &str, expected: ResourceKind) -> Result<Self, ApiError> {
        let reference = Self::parse(value)?;
        if reference.kind != expected {
            return Err(ApiError::InvalidData);
        }
        Ok(reference)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub id: u32,
    pub name: String,
    pub height_cm: Measurement,
    pub mass_kg: Measurement,
    pub birth_year: String,
    pub eye_color: String,
    pub hair_color: String,
    pub homeworld: ResourceRef,
    pub films: Vec<ResourceRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeoplePage {
    pub total: u32,
    pub next_page: Option<u32>,
    pub people: Vec<Person>,
}

#[derive(Deserialize)]
pub(crate) struct PersonDto {
    name: String,
    height: String,
    mass: String,
    birth_year: String,
    eye_color: String,
    hair_color: String,
    homeworld: String,
    films: Vec<String>,
    url: String,
}

impl TryFrom<PersonDto> for Person {
    type Error = ApiError;
    fn try_from(dto: PersonDto) -> Result<Self, Self::Error> {
        if dto.name.trim().is_empty() {
            return Err(ApiError::InvalidData);
        }
        Ok(Self {
            id: ResourceRef::of_kind(&dto.url, ResourceKind::Person)?.id,
            name: dto.name,
            height_cm: Measurement::parse(&dto.height)?,
            mass_kg: Measurement::parse(&dto.mass)?,
            birth_year: dto.birth_year,
            eye_color: dto.eye_color,
            hair_color: dto.hair_color,
            homeworld: ResourceRef::of_kind(&dto.homeworld, ResourceKind::Planet)?,
            films: dto
                .films
                .iter()
                .map(|s| ResourceRef::of_kind(s, ResourceKind::Film))
                .collect::<Result<_, _>>()?,
        })
    }
}
