use std::{
    io::Write,
    net::{IpAddr, SocketAddrV4, SocketAddrV6},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use clap::Parser;
use iroh::{
    dns::DnsResolver,
    endpoint::{Connection, RecvStream, SendStream},
    Endpoint, RelayMode, SecretKey,
};
use tokio::net::TcpStream;
use tokio_rustls::{
    rustls::{
        crypto::{self, CryptoProvider},
        ClientConfig, RootCertStore,
    },
    TlsConnector,
};
use tracing::{error, info};
use tracing_subscriber::{self, EnvFilter};

#[derive(Debug, Parser)]
struct Args {
    #[arg(
        short = 'a',
        long,
        default_value = "0.0.0.0",
        required_if_eq("fly", "false")
    )]
    bind_addr: String,

    #[arg(
        short = 'p',
        long,
        default_value_t = 3002,
        required_if_eq("fly", "false")
    )]
    bind_port: u16,

    #[arg(
        short = 't',
        long,
        default_value = "localhost:3000",
        required_if_eq("fly", "false"),
        help = "Target address:port to forward traffic to"
    )]
    target_uri: String,

    #[arg(long, hide = true, default_value_t = false)]
    fly: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("spacegate_proxy=info"))
        .init();

    let mut args = Args::parse();

    CryptoProvider::install_default(crypto::aws_lc_rs::default_provider())
        .expect("Failed to setup CryptoProvider");

    let endpoint = if args.fly {
        start_server_fly(&mut args).await?
    } else {
        start_server_local(&mut args).await?
    };

    info!(
        "Successfully Initialized\nNode info: {:#?}",
        endpoint.node_addr().await?
    );

    info!("Accepting connections");
    loop {
        let Some(acc) = endpoint.accept().await else {
            error!("Endpoint closed");
            return Ok(());
        };

        let Ok(partial_conn) = acc.accept() else {
            error!("Error accepting");
            continue;
        };

        let Ok(conn) = partial_conn.await else {
            error!("Error completing connection");
            continue;
        };

        let targ = args.target_uri.clone();
        tokio::spawn(async move { handle_conn(conn, targ).await });
    }
}

fn create_key(key: &str) -> [u8; 32] {
    let privkey = key;
    let mut key = [0; 32];
    let mut keybuf: &mut [u8] = &mut key;
    keybuf
        .write_all(privkey.as_bytes())
        .expect("failed to write key");
    key
}

async fn _start_server(args: &Args, alpn: &str, skey: Option<SecretKey>) -> Result<Endpoint> {
    let addr: IpAddr = args.bind_addr.parse()?;

    info!("Using Addr: {}:{}", &addr, args.bind_port);

    let endpoint = Endpoint::builder().alpns(vec![alpn.into()]);

    let endpoint = if let Some(key) = skey {
        endpoint
            .secret_key(key)
            .clear_discovery()
            .relay_mode(RelayMode::Disabled)
    } else {
        endpoint.discovery_n0()
    };

    let endpoint = if let IpAddr::V4(v4) = addr {
        endpoint
            .bind_addr_v4(SocketAddrV4::new(v4, args.bind_port))
            .bind()
            .await?
    } else if let IpAddr::V6(v6) = addr {
        endpoint
            .bind_addr_v6(SocketAddrV6::new(v6, args.bind_port, 0, 0))
            .bind()
            .await?
    } else {
        unreachable!("Failed to bind endpoint");
    };

    Ok(endpoint)
}

async fn start_server_local(args: &mut Args) -> Result<Endpoint> {
    _start_server(args, "stdb", None).await
}

async fn start_server_fly(args: &mut Args) -> Result<Endpoint> {
    args.target_uri = "maincloud.spacetimedb.com:443".into();
    args.bind_addr = "fly-global-services".into();
    args.bind_port = 8080;

    let addr = DnsResolver::new()
        .lookup_ipv4_ipv6(&args.bind_addr, Duration::from_secs(3))
        .await?
        .next()
        .unwrap();
    args.bind_addr = addr.to_string();

    // use a stable key for constant discoverability
    let privkey = create_key("fancykey");
    let key = SecretKey::from_bytes(&privkey);

    _start_server(args, "maincloud", Some(key)).await
}

async fn handle_conn(conn: Connection, target: String) -> Result<()> {
    info!("New connection ID: {}", conn.stable_id());

    loop {
        let Ok((send, recv)) = conn.accept_bi().await else {
            info!("Connection closed for ID: {}", conn.stable_id());
            break;
        };

        let desc = format!("{}:{}", conn.stable_id(), recv.id().index());
        info!("Starting proxy for {}", &desc);

        let targ = target.clone();
        tokio::spawn(async move { proxy_stream(send, recv, targ, desc).await });
    }

    Ok(())
}

async fn proxy_stream(
    mut send: SendStream,
    mut recv: RecvStream,
    target: String,
    desc: String,
) -> Result<()> {
    let stream = TcpStream::connect(&target)
        .await
        .expect("Failed to connect to target server");

    let root_store = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.into(),
    };

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));

    let tls = connector
        .connect("maincloud.spacetimedb.com".try_into().unwrap(), stream)
        .await?;

    let (mut trecv, mut tsend) = tokio::io::split(tls);

    tokio::select! {
        biased;
        _ = tokio::io::copy(&mut recv, &mut tsend) => {},
        _ = tokio::io::copy(&mut trecv, &mut send) => {},
    }

    info!("Proxy Finished for {}", desc);
    Ok(())
}
