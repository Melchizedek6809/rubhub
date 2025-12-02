use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use russh::keys::{Certificate, *};
use russh::server::{Msg, Server as _, Session};
use russh::*;
use sea_orm::EntityTrait;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::state::GlobalState;

pub async fn start_ssh_server(state: GlobalState) -> Result<(), std::io::Error> {
    let key = fs::read_to_string("./data/private_key").expect("You need to generate a keypair first");
    let key = russh::keys::PrivateKey::from_openssh(key).expect("Invalid private key");
    let keys: Vec<PrivateKey> = vec![key];

    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        keys,
        preferred: Preferred {
            // kex: std::borrow::Cow::Owned(vec![russh::kex::DH_GEX_SHA256]),
            ..Preferred::default()
        },
        ..Default::default()
    };
    let config = Arc::new(config);
    let mut sh = Server {
        clients: Arc::new(Mutex::new(HashMap::new())),
        state,
        id: 0,
    };

    let socket = TcpListener::bind(("127.0.0.1", 2222)).await.unwrap();
    let server = sh.run_on_socket(config, &socket);
    let _handle = server.handle();

    println!("Started rubhub SSH server on 2222");

    server.await
}

#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<HashMap<usize, (ChannelId, russh::server::Handle)>>>,
    state: GlobalState,
    id: usize,
}

impl Server {
    async fn post(&mut self, data: CryptoVec) {
        let mut clients = self.clients.lock().await;
        for (id, (channel, s)) in clients.iter_mut() {
            if *id != self.id {
                let _ = s.data(*channel, data.clone()).await;
            }
        }
    }
}

impl server::Server for Server {
    type Handler = Self;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }

    fn handle_session_error(&mut self, _error: <Self::Handler as russh::server::Handler>::Error) {
        eprintln!("Session error: {_error:#?}");
    }
}

impl server::Handler for Server {
    type Error = russh::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        {
            let mut clients = self.clients.lock().await;
            clients.insert(self.id, (channel.id(), session.handle()));
        }
        Ok(true)
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        key: &ssh_key::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        let openssh = key.to_openssh()?;
        println!("Auth publickey: {openssh}");

        let row = crate::entities::ssh_key::Entity::find().one(&self.state.db).await;
        match row {
            Ok(Some(row)) => {
                println!("Row: {row:?}");
                Ok(server::Auth::Accept)
            },
            _ => Err(russh::Error::RequestDenied),
        }
    }

    async fn auth_openssh_certificate(
        &mut self,
        _user: &str,
        certificate: &Certificate,
    ) -> Result<server::Auth, Self::Error> {
        println!("Auth openssh cert: {certificate:?}");
        Ok(server::Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        // Sending Ctrl+C ends the session and disconnects the client
        if data == [3] {
            return Err(russh::Error::Disconnect);
        }

        let data = CryptoVec::from(format!("Got data: {}\r\n", String::from_utf8_lossy(data)));
        self.post(data.clone()).await;
        session.data(channel, data)?;
        Ok(())
    }

    // Disallow IP forwarding
    async fn tcpip_forward(
        &mut self,
        _address: &str,
        _port: &mut u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        Err(russh::Error::RequestDenied)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let id = self.id;
        let clients = self.clients.clone();
        tokio::spawn(async move {
            let mut clients = clients.lock().await;
            clients.remove(&id);
        });
    }
}