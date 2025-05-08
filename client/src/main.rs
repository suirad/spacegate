use std::{net::SocketAddr, str::FromStr};

use anyhow::Result;
use iroh::{Endpoint, NodeAddr, PublicKey, RelayMode};
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
    // clean this up
    // add cli
    let listen_uri = "0.0.0.0:3001".to_string();

    let listener = tokio::net::TcpListener::bind(&listen_uri).await?;
    println!("Listening on: {}", listen_uri);

    loop {
        let (mut stream, _addr) = listener.accept().await?;

        let _: JoinHandle<Result<()>> = tokio::spawn(async move {
            let (mut recv, mut send) = stream.split();

            let endpoint = Endpoint::builder()
                .relay_mode(RelayMode::Disabled)
                .bind()
                .await?;

            println!(
                "Successfully Initialized, node id: {:#?}",
                endpoint.node_addr().await?
            );
            let proxy_id = "49da895f8923d4d5c56029de25330a5e70f954a018fa6c49a462b795dd7c1915";
            let pubkey = PublicKey::from_str(proxy_id)?;

            println!("Starting connection...");
            let servernode = NodeAddr::new(pubkey)
                .with_direct_addresses([SocketAddr::from_str("0.0.0.0:8080")?]);
            let conn = endpoint.connect(servernode, "maincloud".as_bytes()).await?;

            println!("Creating stream...");
            let (mut prox_send, mut prox_recv) = conn.open_bi().await?;

            println!("Successfully connected to proxy! ID: {}", conn.stable_id());

            tokio::select! {
                _ = tokio::io::copy(&mut recv, &mut prox_send) => {},
                _ = tokio::io::copy(&mut prox_recv, &mut send) => {},
            }
            Ok(())
        });
    }
}
