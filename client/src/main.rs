use core::str;
use std::{net::SocketAddr, str::FromStr, time::Duration};

use anyhow::Result;
use clap::Parser;
use iroh::{dns::DnsResolver, endpoint::SendStream, Endpoint, NodeAddr, PublicKey, RelayMode};
use tokio::{io::AsyncReadExt, net::tcp::ReadHalf, task::JoinHandle};
use tracing::info;
use tracing_subscriber::{self, EnvFilter};

#[derive(Debug, Parser, Clone)]
struct Args {
    #[arg(
        short,
        long,
        help = "The forward proxy node to connect to (forward proxy prints it on start)"
    )]
    node: Option<String>,
    #[arg(short = 'a', long, default_value = "0.0.0.0")]
    bind_addr: String,
    #[arg(short = 'p', long, default_value_t = 3001)]
    bind_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("spacegate=info"))
        .init();

    let args = Args::parse();

    let listen_uri = format!("{}:{}", args.bind_addr, args.bind_port);
    let servernode = if let Some(ref node) = args.node {
        let pubkey = PublicKey::from_str(node)?;
        NodeAddr::new(pubkey)
    } else {
        let proxy_id = "49da895f8923d4d5c56029de25330a5e70f954a018fa6c49a462b795dd7c1915";
        let pubkey = PublicKey::from_str(proxy_id)?;
        let mut addrs = DnsResolver::new()
            .lookup_ipv4_ipv6("stdb-iroh.fly.dev", Duration::from_secs(3))
            .await?;
        let proxy_addr = addrs.next().unwrap();

        NodeAddr::new(pubkey).with_direct_addresses([SocketAddr::new(proxy_addr, 8080)])
    };

    let listener = tokio::net::TcpListener::bind(&listen_uri).await?;
    info!("Listening on: {}", listen_uri);

    info!("Starting connection...");
    let endpoint = Endpoint::builder();

    let conn = if args.node.is_some() {
        let ep = endpoint.discovery_n0().bind().await?;
        info!(
            "Successfully Initialized, node id: {:#?}",
            ep.node_addr().await?
        );

        ep.connect(servernode, "stdb".as_bytes()).await?
    } else {
        let ep = endpoint.relay_mode(RelayMode::Disabled).bind().await?;
        info!(
            "Successfully Initialized, node id: {:#?}",
            ep.node_addr().await?
        );

        ep.connect(servernode, "maincloud".as_bytes()).await?
    };

    info!("Established proxy connection!");

    loop {
        let (mut stream, _addr) = listener.accept().await?;

        info!("Creating stream...");
        let (mut prox_send, mut prox_recv) = conn.open_bi().await?;
        let conid = conn.stable_id();

        let args_copy = args.clone();
        let _task: JoinHandle<Result<()>> = tokio::spawn(async move {
            let (mut recv, mut send) = stream.split();

            if args_copy.node.is_none() {
                let target_host = "maincloud.spacetimedb.com";
                if let Err(e) = rewrite_host_header(&mut recv, &mut prox_send, target_host).await {
                    panic!("Error parsing host header out of request: {e}");
                }
            }

            info!("Proxying connection with ID: {}", conid);

            tokio::select! {
                _ = tokio::io::copy(&mut recv, &mut prox_send) => {},
                _ = tokio::io::copy(&mut prox_recv, &mut send) => {},
            }
            Ok(())
        });
    }
}

async fn rewrite_host_header(
    recv: &mut ReadHalf<'_>,
    prox_send: &mut SendStream,
    new_host: &str,
) -> Result<()> {
    let mut buf = [0u8; 200];

    let len = recv.read(&mut buf).await?;

    let data = str::from_utf8(&buf[0..len])?;

    if let Some((pre_host, post_host)) = data.split_once("Host: ") {
        let chunks = [
            pre_host.as_bytes(),
            "Host: ".as_bytes(),
            new_host.as_bytes(),
            "\n".as_bytes(),
            post_host.split_once('\n').unwrap().1.as_bytes(),
        ];

        for chunk in chunks {
            prox_send.write_all(chunk).await?;
        }
    } else if let Some((pre_host, post_host)) = data.split_once("host: ") {
        let chunks = [
            pre_host.as_bytes(),
            "host: ".as_bytes(),
            new_host.as_bytes(),
            "\n".as_bytes(),
            post_host.split_once('\n').unwrap().1.as_bytes(),
        ];

        for chunk in chunks {
            prox_send.write_all(chunk).await?;
        }
    } else {
        prox_send.write_all(&buf[0..len]).await?;
    }

    Ok(())
}
