use std::{io::Write, net::SocketAddrV4, str::FromStr};

use anyhow::Result;
use iroh::{
    endpoint::{Connection, RecvStream, SendStream},
    Endpoint, SecretKey,
};

#[tokio::main]
async fn main() -> Result<()> {
    // cli
    let target = "localhost:3000".to_string();
    let privkey = create_key("fancykey");

    let endpoint = Endpoint::builder()
        .secret_key(SecretKey::from_bytes(&privkey))
        .alpns(vec!["maincloud".into()])
        .clear_discovery()
        .relay_mode(iroh::RelayMode::Disabled)
        .bind_addr_v4(SocketAddrV4::from_str("0.0.0.0:8080")?)
        .bind()
        .await?;

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
    let mut target_conn = tokio::net::TcpStream::connect(target).await?;
    let (mut trecv, mut tsend) = target_conn.split();

    tokio::select! {
        biased;
        _ = tokio::io::copy(&mut recv, &mut tsend) => {},
        _ = tokio::io::copy(&mut trecv, &mut send) => {},
    }

    println!("Proxy Finished for {}", desc);
    Ok(())
}
