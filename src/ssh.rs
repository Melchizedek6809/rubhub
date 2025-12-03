use std::fs;
use std::process::Stdio;
use std::sync::Arc;

use russh::keys::{Certificate, *};
use russh::server::{Msg, Server as _, Session};
use russh::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::process::Command;
use uuid::Uuid;

use crate::state::GlobalState;

use crate::entities::ssh_key as db_ssh_key;

pub async fn start_ssh_server(state: GlobalState) -> Result<(), std::io::Error> {
    let key =
        fs::read_to_string("./data/private_key").expect("You need to generate a keypair first");
    let key = russh::keys::PrivateKey::from_openssh(key).expect("Invalid private key");
    let keys: Vec<PrivateKey> = vec![key];

    let mut methods = MethodSet::empty();
    methods.push(MethodKind::PublicKey);

    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(10)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        keys,
        methods,
        preferred: Preferred {
            // kex: std::borrow::Cow::Owned(vec![russh::kex::DH_GEX_SHA256]),
            ..Preferred::default()
        },
        ..Default::default()
    };
    let config = Arc::new(config);
    let mut sh = Server { state };

    let socket = TcpListener::bind(("127.0.0.1", 2222)).await.unwrap();
    let server = sh.run_on_socket(config, &socket);
    let _handle = server.handle();

    println!("Started rubhub SSH server on 2222");

    server.await
}

#[derive(Clone)]
struct Server {
    state: GlobalState,
}

struct Connection {
    handle: Option<russh::server::Handle>,
    channel_id: Option<russh::ChannelId>,
    sender_to_git: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,

    state: GlobalState,
    user_id: Option<Uuid>,
}

impl Connection {
    async fn handle_upload_pack(
        &mut self,
        path: String,
        rx_from_ssh: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> Result<(), russh::Error> {
        self.handle_with_command("git-upload-pack".to_string(), path, rx_from_ssh)
            .await
    }

    async fn handle_receive_pack(
        &mut self,
        path: String,
        rx_from_ssh: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> Result<(), russh::Error> {
        self.handle_with_command("git-receive-pack".to_string(), path, rx_from_ssh)
            .await
    }

    async fn handle_archive_pack(
        &mut self,
        path: String,
        rx_from_ssh: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> Result<(), russh::Error> {
        self.handle_with_command("git-upload-archive".to_string(), path, rx_from_ssh)
            .await
    }

    async fn handle_with_command(
        &mut self,
        command: String,
        path: String,
        mut rx_from_ssh: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> Result<(), russh::Error> {
        let path = self.state.config.git_root.join(path);

        let handle = self.handle.clone().unwrap();
        let id = self.channel_id.unwrap();

        let mut child = Command::new(command)
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut git_stdin = child.stdin.take().unwrap();
        // task: SSH → git stdin
        tokio::spawn(async move {
            while let Some(data) = rx_from_ssh.recv().await {
                // println!("<- {}", String::from_utf8_lossy(&data));
                if git_stdin.write_all(&data).await.is_err() {
                    break;
                }
            }
            let _ = git_stdin.shutdown().await;
        });

        // task: git stdout → SSH
        let mut git_stdout = child.stdout.take().unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            loop {
                let n = match git_stdout.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                // println!("-> {}", String::from_utf8_lossy(&buf[..n]));

                if handle
                    .data(id, CryptoVec::from_slice(&buf[..n]))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            let _ = handle.eof(id).await;
            let _ = handle.exit_status_request(id, 0).await.ok();
            let _ = handle.close(id).await;
        });

        Ok(())
    }
}

impl server::Server for Server {
    type Handler = Connection;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Connection {
        Connection {
            state: self.state.clone(),
            user_id: None,
            channel_id: None,
            handle: None,
            sender_to_git: None,
        }
    }

    fn handle_session_error(&mut self, _error: <Self::Handler as russh::server::Handler>::Error) {
        eprintln!("Session error: {_error:#?}");
    }
}

impl server::Handler for Connection {
    type Error = russh::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        match self.user_id {
            Some(user_id) => {
                let user = crate::entities::user::Entity::find_by_id(user_id)
                    .one(&self.state.db)
                    .await;

                match user {
                    Ok(Some(_user)) => {
                        self.handle = Some(session.handle());
                        self.channel_id = Some(channel.id());
                        Ok(true)
                    }
                    _ => Err(russh::Error::NoAuthMethod),
                }
            }
            None => Err(russh::Error::NoAuthMethod),
        }
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        key: &ssh_key::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        let openssh = key.to_openssh()?;
        println!("Auth publickey: {openssh}");

        let row = db_ssh_key::Entity::find()
            .filter(db_ssh_key::Column::PublicKey.eq(&openssh))
            .one(&self.state.db)
            .await;

        match row {
            Ok(Some(row)) => {
                self.user_id = Some(row.user_id);
                println!("Auth: {}", row.user_id);
                Ok(server::Auth::Accept)
            }
            _ => Err(russh::Error::RequestDenied),
        }
    }

    async fn auth_openssh_certificate(
        &mut self,
        _user: &str,
        certificate: &Certificate,
    ) -> Result<server::Auth, Self::Error> {
        println!("Auth openssh cert: {certificate:?}");
        Err(russh::Error::NoAuthMethod)
    }

    async fn exec_request(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let cmdline = String::from_utf8_lossy(data);
        let parts = cmdline.split_ascii_whitespace().collect::<Vec<&str>>();

        println!("Exec: {parts:?}\r\n",);

        if parts.len() < 2 {
            Err(russh::Error::RequestDenied)
        } else {
            let path = parts[1];
            let path = path.trim_start_matches("'").trim_end_matches("'");
            let path = path.to_string();

            let (tx, rx) = tokio::sync::mpsc::channel(16);
            self.sender_to_git = Some(tx);

            match parts[0] {
                "git-upload-pack" => self.handle_upload_pack(path, rx).await,
                "git-receive-pack" => self.handle_receive_pack(path, rx).await,
                "git-upload-archive" => self.handle_archive_pack(path, rx).await,
                _ => Err(russh::Error::RequestDenied),
            }
        }
    }

    async fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        // Sending Ctrl+C ends the session and disconnects the client
        if let Some(tx) = &self.sender_to_git {
            let err = tx.send(data.to_vec()).await;
            if err.is_err() {
                Err(russh::Error::Disconnect)
            } else {
                Ok(())
            }
        } else {
            println!("We only support git for now");
            Err(russh::Error::Disconnect)
        }
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
