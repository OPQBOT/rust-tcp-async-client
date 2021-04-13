use failure::Error;
use rust_network::server::{tcp_run, Server};
use rust_network::stats::Stats;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let server = Server::new();
    let stats = Stats::new();
    tcp_run(&server, "0.0.0.0:8080".parse().unwrap(), stats, 100)
        .await
        .map_err(Error::from)
}
