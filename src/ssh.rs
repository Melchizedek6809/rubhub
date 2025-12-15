use std::path::PathBuf;
use std::{fs, io, process::Stdio, sync::Arc};

use russh::keys::*;
use russh::server::{Msg, Server as _, Session};
use russh::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpSocket;
use tokio::process::Command;
use tokio::task::JoinHandle;

use crate::{AccessType, GlobalState, Project, User};

async fn ensure_host_key(path: &PathBuf, key_type: &str) -> Result<(), io::Error> {
    if path.exists() {
        return Ok(());
    }

    println!("Generating missing {key_type} host key");
    let status = Command::new("ssh-keygen")
        .arg("-t")
        .arg(key_type)
        .arg("-N")
        .arg("")
        .arg("-f")
        .arg(path)
        .status()
        .await?;

    if status.success() {
        eprintln!("No {key_type} SSH key found, generated one using ssh-keygen");
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "ssh-keygen failed for {key_type}"
        )))
    }
}

async fn load_or_create_key(state: &GlobalState, key_type: &str) -> Result<PrivateKey, io::Error> {
    let filename = format!("id_{key_type}");
    let path = state.config.dir_root.join(filename);
    ensure_host_key(&path, key_type).await?;
    let ed_key = fs::read_to_string(path)?;
    russh::keys::PrivateKey::from_openssh(ed_key).map_err(io::Error::other)
}

pub async fn ssh_server(
    state: GlobalState,
) -> Result<JoinHandle<Result<(), std::io::Error>>, std::io::Error> {
    let ed_key = load_or_create_key(&state, "ed25519").await?;
    let rsa_key = load_or_create_key(&state, "rsa").await?;

    let mut methods = MethodSet::empty();
    methods.push(MethodKind::PublicKey);

    let config = russh::server::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(10)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        keys: vec![ed_key, rsa_key],
        methods,
        preferred: Preferred {
            // kex: std::borrow::Cow::Owned(vec![russh::kex::DH_GEX_SHA256]),
            ..Preferred::default()
        },
        ..Default::default()
    };
    let config = Arc::new(config);
    let process_start = state.process_start;
    let mut sh = Server {
        state: state.clone(),
    };

    let bind_addr = sh.state.config.ssh_bind_addr;
    let socket = if bind_addr.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };
    socket.set_reuseaddr(true)?;
    #[cfg(target_os = "linux")]
    socket.set_reuseport(state.config.reuse_port)?;
    socket.bind(bind_addr)?;
    let socket = socket.listen(1024)?;

    println!(
        "[{:?}] - Started rubhub SSH server on {bind_addr}",
        process_start.elapsed()
    );

    Ok(tokio::task::spawn(async move {
        let server = sh.run_on_socket(config, &socket);
        let _handle = server.handle();

        server.await
    }))
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
    user_slug: Option<String>,
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

        let handle = self.handle.clone().ok_or(russh::Error::SendError)?;
        let id = self.channel_id.ok_or(russh::Error::SendError)?;

        let mut child = Command::new(command)
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut git_stdin = child.stdin.take().ok_or(russh::Error::SendError)?;
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
        let mut git_stdout = child.stdout.take().ok_or(russh::Error::SendError)?;
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
            user_slug: None,
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
        if let Some(user_slug) = &self.user_slug {
            let user = User::load(&self.state, user_slug).await;

            if user.is_err() {
                return Err(russh::Error::NoAuthMethod);
            }
        }

        self.handle = Some(session.handle());
        self.channel_id = Some(channel.id());
        Ok(true)
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &ssh_key::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        let openssh = key.to_openssh()?;

        if user == "anon" {
            self.user_slug = None;
            return Ok(server::Auth::Accept);
        }

        match User::load(&self.state, user).await {
            Ok(user) => match user.validate_ssh_key(key) {
                Ok(_) => {
                    println!("SSH Accept: {} - {openssh}", user.slug);
                    self.user_slug = Some(user.slug);
                    return Ok(server::Auth::Accept);
                }
                Err(_e) => {
                    self.user_slug = None;
                    println!("SSH Reject: {} - {openssh}", user.slug);
                }
            },
            Err(_) => {
                self.user_slug = None;
                println!("Anon Auth - PK {openssh}");
            }
        }
        Ok(server::Auth::Reject {
            partial_success: false,
            proceed_with_methods: None,
        })
    }

    async fn exec_request(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let cmdline = String::from_utf8_lossy(data);
        let parts = cmdline.split_ascii_whitespace().collect::<Vec<&str>>();

        if parts.len() < 2 {
            return Err(russh::Error::RequestDenied);
        }

        let Some(command) = parts.first() else {
            return Err(russh::Error::RequestDenied);
        };

        let required_access = match required_access_for_command(command) {
            Some(access) => access,
            None => return Err(russh::Error::RequestDenied),
        };

        let path = parts[1];
        let path = path.trim_start_matches('\'').trim_end_matches('\'');
        let path = path.trim_start_matches('/').trim_end_matches('/');
        let path = path.to_string();

        let Ok((owner, project)) = Project::load_by_path(&self.state, path).await else {
            return Err(russh::Error::RequestDenied);
        };

        let access_level = project.access_level(self.user_slug.clone()).await;

        if !has_required_access(access_level, required_access) {
            eprintln!(
                "SSH access denied: user {:?} requested {command} on {}/{} (has {access_level:?}, needs {required_access:?})",
                self.user_slug, owner.slug, project.slug
            );
            return Err(russh::Error::RequestDenied);
        }

        let repo_path = format!("{}/{}", owner.slug, project.slug);

        let (tx, rx) = tokio::sync::mpsc::channel(16);
        self.sender_to_git = Some(tx);

        match *command {
            "git-upload-pack" => self.handle_upload_pack(repo_path, rx).await,
            "git-receive-pack" => self.handle_receive_pack(repo_path, rx).await,
            "git-upload-archive" => self.handle_archive_pack(repo_path, rx).await,
            _ => Err(russh::Error::RequestDenied),
        }
    }

    async fn authentication_banner(&mut self) -> Result<Option<String>, Self::Error> {
        Ok(Some(
            "Welcome to rubhub.net

If you see \"Permission denied (publickey)\", generate an SSH key first:

    ssh-keygen -t ed25519

You do NOT need an account, any key works for anonymous access.\r\n\r\n"
                .to_string(),
        ))
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

fn required_access_for_command(command: &str) -> Option<AccessType> {
    match command {
        "git-upload-pack" | "git-upload-archive" => Some(AccessType::Read),
        "git-receive-pack" => Some(AccessType::Write),
        _ => None,
    }
}

fn has_required_access(current: AccessType, required: AccessType) -> bool {
    match required {
        AccessType::None => true,
        AccessType::Read => matches!(
            current,
            AccessType::Read | AccessType::Write | AccessType::Admin
        ),
        AccessType::Write => matches!(current, AccessType::Write | AccessType::Admin),
        AccessType::Admin => matches!(current, AccessType::Admin),
    }
}
