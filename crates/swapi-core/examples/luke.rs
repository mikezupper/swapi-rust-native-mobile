use swapi_core::SwapiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let luke = SwapiClient::new()?.person(1).await?;
    println!(
        "{}: {:?} cm, {} films",
        luke.name,
        luke.height_cm,
        luke.films.len()
    );
    Ok(())
}
