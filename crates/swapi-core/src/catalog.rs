//! Validated catalog records and a shared pager for every SWAPI collection.
use crate::people::{Page, PageSnapshot, PagedStore, Record};
use crate::{ApiError, Loaded, ResourceKind, ResourceRef, SwapiClient};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::sync::Arc;
use tokio::runtime::Handle;

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceFact {
    pub label: String,
    pub value: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CatalogRecord {
    pub id: u32,
    pub kind: ResourceKind,
    pub name: String,
    pub facts: Vec<ResourceFact>,
    pub description: Option<String>,
}
impl Record for CatalogRecord {
    fn id(&self) -> u32 {
        self.id
    }
}
pub type CatalogSnapshot = PageSnapshot<CatalogRecord>;
pub type CatalogStore = PagedStore<CatalogRecord>;

impl CatalogStore {
    /// A detail session uses the same cancellation/retry/lifetime machinery as
    /// a collection, with a single immutable resource identity.
    pub fn catalog(
        client: SwapiClient,
        kind: ResourceKind,
        id: Option<u32>,
        runtime: &Handle,
    ) -> Self {
        let cached = client.clone();
        Self::start_cached(
            Arc::new(move |query, page, force| {
                let client = client.clone();
                Box::pin(async move {
                    match id {
                        Some(id) => client.resource_loaded(kind, id, force).await.map(|loaded| {
                            loaded.map(|record| Page {
                                total: 1,
                                next_page: None,
                                items: vec![record],
                            })
                        }),
                        None => client.resources_loaded(kind, &query, page, force).await,
                    }
                })
            }),
            Arc::new(move |query, page| match id {
                Some(id) => cached.cached_resource(kind, id).map(|loaded| {
                    loaded.map(|record| Page {
                        total: 1,
                        next_page: None,
                        items: vec![record],
                    })
                }),
                None => cached.cached_resources(kind, query, page),
            }),
            runtime,
        )
    }
}

#[derive(Deserialize)]
struct CatalogPageDto {
    count: u32,
    next: Option<String>,
    results: Vec<Value>,
}

impl SwapiClient {
    pub async fn resources(
        &self,
        kind: ResourceKind,
        query: &str,
        page: u32,
    ) -> Result<Page<CatalogRecord>, ApiError> {
        Ok(self.resources_loaded(kind, query, page, false).await?.value)
    }
    pub(crate) fn resources_url(
        &self,
        kind: ResourceKind,
        query: &str,
        page: u32,
    ) -> Result<url::Url, ApiError> {
        if page == 0 {
            return Err(ApiError::InvalidRequest);
        }
        let query = query.trim();
        let mut url = self
            .base
            .join(&format!("{}/", kind.path()))
            .map_err(|_| ApiError::InvalidRequest)?;
        url.query_pairs_mut().append_pair("page", &page.to_string());
        if !query.is_empty() {
            url.query_pairs_mut().append_pair("search", query);
        }
        Ok(url)
    }
    fn parse_resources(
        &self,
        kind: ResourceKind,
        query: &str,
        page: u32,
        value: Value,
    ) -> Result<Page<CatalogRecord>, ApiError> {
        let dto: CatalogPageDto =
            serde_json::from_value(value).map_err(|_| ApiError::InvalidData)?;
        let next_page = dto
            .next
            .as_deref()
            .map(|s| self.next_page(s, kind, query, page))
            .transpose()?;
        let items = dto
            .results
            .into_iter()
            .map(|v| CatalogRecord::parse(kind, v))
            .collect::<Result<Vec<_>, _>>()?;
        if items.len() > dto.count as usize {
            return Err(ApiError::InvalidData);
        }
        Ok(Page {
            total: dto.count,
            next_page,
            items,
        })
    }
    pub(crate) async fn resources_loaded(
        &self,
        kind: ResourceKind,
        query: &str,
        page: u32,
        force: bool,
    ) -> Result<Loaded<Page<CatalogRecord>>, ApiError> {
        self.load(self.resources_url(kind, query, page)?, force, |value| {
            self.parse_resources(kind, query.trim(), page, value)
        })
        .await
    }
    pub(crate) fn cached_resources(
        &self,
        kind: ResourceKind,
        query: &str,
        page: u32,
    ) -> Option<Loaded<Page<CatalogRecord>>> {
        self.cached(&self.resources_url(kind, query, page).ok()?, |value| {
            self.parse_resources(kind, query.trim(), page, value)
        })
    }
    pub async fn resource(&self, kind: ResourceKind, id: u32) -> Result<CatalogRecord, ApiError> {
        Ok(self.resource_loaded(kind, id, false).await?.value)
    }
    fn resource_url(&self, kind: ResourceKind, id: u32) -> Result<url::Url, ApiError> {
        if id == 0 {
            return Err(ApiError::InvalidRequest);
        }
        self.base
            .join(&format!("{}/{id}/", kind.path()))
            .map_err(|_| ApiError::InvalidRequest)
    }
    fn parse_resource(
        kind: ResourceKind,
        id: u32,
        value: Value,
    ) -> Result<CatalogRecord, ApiError> {
        let record = CatalogRecord::parse(kind, value)?;
        if record.id != id {
            return Err(ApiError::InvalidData);
        }
        Ok(record)
    }
    pub(crate) async fn resource_loaded(
        &self,
        kind: ResourceKind,
        id: u32,
        force: bool,
    ) -> Result<Loaded<CatalogRecord>, ApiError> {
        self.load(self.resource_url(kind, id)?, force, |value| {
            Self::parse_resource(kind, id, value)
        })
        .await
    }
    pub(crate) fn cached_resource(
        &self,
        kind: ResourceKind,
        id: u32,
    ) -> Option<Loaded<CatalogRecord>> {
        self.cached(&self.resource_url(kind, id).ok()?, |value| {
            Self::parse_resource(kind, id, value)
        })
    }
}

fn field<T: DeserializeOwned>(value: &Value, key: &str) -> Result<T, ApiError> {
    serde_json::from_value(value.get(key).ok_or(ApiError::InvalidData)?.clone())
        .map_err(|_| ApiError::InvalidData)
}
impl CatalogRecord {
    pub(crate) fn parse(kind: ResourceKind, value: Value) -> Result<Self, ApiError> {
        let identity = ResourceRef::parse(&field::<String>(&value, "url")?)?;
        if identity.kind != kind {
            return Err(ApiError::InvalidData);
        }
        let name = field::<String>(
            &value,
            if kind == ResourceKind::Film {
                "title"
            } else {
                "name"
            },
        )?;
        if name.trim().is_empty() {
            return Err(ApiError::InvalidData);
        }
        // Labels are shared English display metadata; native code owns their
        // layout, styling and accessibility. API sentinel strings stay intact.
        let fields: &[(&str, &str)] = match kind {
            ResourceKind::Person => &[
                ("birth_year", "Born"),
                ("height", "Height (cm)"),
                ("mass", "Mass (kg)"),
                ("eye_color", "Eye color"),
                ("hair_color", "Hair color"),
            ],
            ResourceKind::Planet => &[
                ("climate", "Climate"),
                ("terrain", "Terrain"),
                ("population", "Population"),
                ("diameter", "Diameter (km)"),
                ("gravity", "Gravity (standard G)"),
                ("rotation_period", "Rotation (hours)"),
                ("orbital_period", "Orbit (days)"),
                ("surface_water", "Surface water (%)"),
            ],
            ResourceKind::Starship => &[
                ("model", "Model"),
                ("starship_class", "Class"),
                ("manufacturer", "Manufacturer"),
                ("hyperdrive_rating", "Hyperdrive rating"),
                ("MGLT", "Megalights per hour"),
                ("crew", "Crew"),
                ("passengers", "Passengers"),
                ("length", "Length (m)"),
                ("cost_in_credits", "Cost (credits)"),
                ("cargo_capacity", "Cargo (kg)"),
                ("max_atmosphering_speed", "Atmospheric speed"),
                ("consumables", "Supplies"),
            ],
            ResourceKind::Vehicle => &[
                ("model", "Model"),
                ("vehicle_class", "Class"),
                ("manufacturer", "Manufacturer"),
                ("crew", "Crew"),
                ("passengers", "Passengers"),
                ("length", "Length (m)"),
                ("cost_in_credits", "Cost (credits)"),
                ("cargo_capacity", "Cargo (kg)"),
                ("max_atmosphering_speed", "Atmospheric speed"),
                ("consumables", "Supplies"),
            ],
            ResourceKind::Species => &[
                ("classification", "Classification"),
                ("designation", "Designation"),
                ("language", "Language"),
                ("average_height", "Average height (cm)"),
                ("average_lifespan", "Average lifespan (years)"),
                ("skin_colors", "Skin colors"),
                ("hair_colors", "Hair colors"),
                ("eye_colors", "Eye colors"),
            ],
            ResourceKind::Film => &[
                ("release_date", "Released"),
                ("director", "Director"),
                ("producer", "Producers"),
            ],
        };
        let mut facts = fields
            .iter()
            .map(|(key, label)| {
                Ok(ResourceFact {
                    label: (*label).into(),
                    value: field(&value, key)?,
                })
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        let description = if kind == ResourceKind::Film {
            let episode: u32 = field(&value, "episode_id")?;
            if episode == 0 {
                return Err(ApiError::InvalidData);
            }
            facts.insert(
                0,
                ResourceFact {
                    label: "Episode".into(),
                    value: episode.to_string(),
                },
            );
            Some(field::<String>(&value, "opening_crawl")?.replace("\r\n", "\n"))
        } else {
            None
        };
        if matches!(kind, ResourceKind::Person | ResourceKind::Species) {
            let world: Option<String> = field(&value, "homeworld")?;
            if let Some(world) = world {
                if ResourceRef::parse(&world)?.kind != ResourceKind::Planet {
                    return Err(ApiError::InvalidData);
                }
            } else if kind == ResourceKind::Person {
                return Err(ApiError::InvalidData);
            }
        }
        let relationships: &[(&str, ResourceKind, &str)] = match kind {
            ResourceKind::Person => &[("films", ResourceKind::Film, "Film appearances")],
            ResourceKind::Planet => &[
                ("films", ResourceKind::Film, "Film appearances"),
                ("residents", ResourceKind::Person, "Residents"),
            ],
            ResourceKind::Starship | ResourceKind::Vehicle => &[
                ("films", ResourceKind::Film, "Film appearances"),
                ("pilots", ResourceKind::Person, "Pilots"),
            ],
            ResourceKind::Species => &[
                ("films", ResourceKind::Film, "Film appearances"),
                ("people", ResourceKind::Person, "People"),
            ],
            ResourceKind::Film => &[
                ("characters", ResourceKind::Person, "Characters"),
                ("planets", ResourceKind::Planet, "Planets"),
                ("starships", ResourceKind::Starship, "Starships"),
                ("vehicles", ResourceKind::Vehicle, "Vehicles"),
                ("species", ResourceKind::Species, "Species"),
            ],
        };
        for (key, expected, label) in relationships {
            let links: Vec<String> = field(&value, key)?;
            for link in &links {
                if ResourceRef::parse(link)?.kind != *expected {
                    return Err(ApiError::InvalidData);
                }
            }
            facts.push(ResourceFact {
                label: (*label).into(),
                value: links.len().to_string(),
            });
        }
        Ok(Self {
            id: identity.id,
            kind,
            name,
            facts,
            description,
        })
    }
}
