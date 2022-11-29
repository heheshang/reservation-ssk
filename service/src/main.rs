use std::path::Path;

use abi::{reservation_service_server::ReservationServiceServer, Config};
use anyhow::Result;
use reservation_service::RsvpService;

#[tokio::main]
async fn main() -> Result<()> {
    // we would first try RESERVATION_CONFIG env var,
    // then try "./reservation.yml",then try "~/.config/reservation.yml"
    // then try "/etc/reservation.yml"
    let filename = std::env::var("RESERVATION_CONFIG").unwrap_or_else(|_| {
        let p1 = Path::new("./reservation.yml");
        let path = shellexpand::tilde("~/.config/reservation.yml");
        let p2 = Path::new(path.as_ref());
        let p3 = Path::new("/etc/reservation.yml");
        match (p1.exists(), p2.exists(), p3.exists()) {
            (true, _, _) => p1.to_str().unwrap().to_string(),
            (_, true, _) => p2.to_str().unwrap().to_string(),
            (_, _, true) => p3.to_str().unwrap().to_string(),
            _ => panic!("no config file found"),
        }
    });
    println!("config file: {}", filename);
    let config = Config::load(&filename)?;
    print!("{:?}", config);
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let svc = RsvpService::from_config(&config).await?;
    let svc = ReservationServiceServer::new(svc);
    tonic::transport::Server::builder()
        .add_service(svc)
        .serve(addr.parse()?)
        .await?;
    Ok(())
}
