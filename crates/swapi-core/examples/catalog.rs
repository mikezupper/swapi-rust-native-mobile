//! Explicit live smoke check; not part of the deterministic test suite.
use swapi_core::{ResourceKind, SwapiClient};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = SwapiClient::new()?;
    for kind in [
        ResourceKind::Person,
        ResourceKind::Planet,
        ResourceKind::Starship,
        ResourceKind::Vehicle,
        ResourceKind::Species,
        ResourceKind::Film,
    ] {
        let page = client.resources(kind, "", 1).await?;
        let first = page.items.first().ok_or("Empty collection")?;
        let detail = client.resource(kind, first.id).await?;
        println!(
            "{}: {} records; detail: {} ({} facts)",
            kind.path(),
            page.total,
            detail.name,
            detail.facts.len()
        );
    }
    Ok(())
}
