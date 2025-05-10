use std::{
    io::Write,
    net::{IpAddr, SocketAddrV4, SocketAddrV6},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use iroh::{
    dns::DnsResolver,
    endpoint::{Connection, RecvStream, SendStream},
    Endpoint, SecretKey,
};
use tokio::net::TcpStream;
use tokio_rustls::{
    rustls::{
        crypto::{self, CryptoProvider},
        ClientConfig, RootCertStore,
    },
    TlsConnector,
};

#[tokio::main]
async fn main() -> Result<()> {
    // cli

    CryptoProvider::install_default(crypto::aws_lc_rs::default_provider())
        .expect("Failed to setup CryptoProvider");

    let target = "maincloud.spacetimedb.com:443".to_string();
    let privkey = create_key("fancykey");

    // resolve fly-global-services
    let addr = DnsResolver::new()
        .lookup_ipv4_ipv6("fly-global-services", Duration::from_secs(3))
        .await?
        .next()
        .unwrap();
    //let addr: IpAddr = "0.0.0.0:8080".parse()?;

    println!("Using Addr: {}", &addr);

    let endpoint = Endpoint::builder()
        .secret_key(SecretKey::from_bytes(&privkey))
        .alpns(vec!["maincloud".into()])
        .clear_discovery()
        .relay_mode(iroh::RelayMode::Disabled);

    let endpoint = if let IpAddr::V4(v4) = addr {
        endpoint
            .bind_addr_v4(SocketAddrV4::new(v4, 8080))
            .bind()
            .await?
    } else if let IpAddr::V6(v6) = addr {
        endpoint
            .bind_addr_v6(SocketAddrV6::new(v6, 8080, 0, 0))
            .bind()
            .await?
    } else {
        unreachable!("Failed to bind endpoint");
    };

    println!(
        "Successfully Initialized\nNode addr: {:#?}",
        endpoint.node_addr().await?
    );

    println!("Accepting connections");
    loop {
        let Some(acc) = endpoint.accept().await else {
            eprintln!("Endpoint closed");
            return Ok(());
        };

        let Ok(partial_conn) = acc.accept() else {
            eprintln!("Error accepting");
            continue;
        };

        let Ok(conn) = partial_conn.await else {
            eprintln!("Error completing connection");
            continue;
        };

        let targ = target.clone();
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

async fn handle_conn(conn: Connection, target: String) -> Result<()> {
    println!("New connection ID: {}", conn.stable_id());

    loop {
        let Ok((send, recv)) = conn.accept_bi().await else {
            println!("Connection closed for ID: {}", conn.stable_id());
            break;
        };

        let desc = format!("{}:{}", conn.stable_id(), recv.id().index());
        println!("Starting proxy for {}", &desc);

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

    println!("Proxy Finished for {}", desc);
    Ok(())
}
