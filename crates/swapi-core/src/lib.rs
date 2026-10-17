//! Shared SWAPI transport and validated records. No platform UI dependencies.
mod client;
mod model;
mod people;
mod state;

pub use client::SwapiClient;
pub use model::{ApiError, Measurement, PeoplePage, Person, ResourceKind, ResourceRef};
pub use people::{PeopleOperation, PeoplePhase, PeopleSnapshot, PeopleStore};
pub use state::{CharacterSnapshot, CharacterState, CharacterStore};
