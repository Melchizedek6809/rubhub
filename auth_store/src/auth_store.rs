use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::mpsc::SendError;
use std::sync::{Arc,mpsc};
use std::thread;
use dashmap::DashMap;
use uuid::Uuid;

use crate::event::StoreEvent;
use crate::{PasswordVerification, User};
use crate::Session;
use crate::SshKey;

#[derive(Clone, Debug)]
pub struct AuthStore {
    chan: mpsc::SyncSender<StoreEvent>,

    user_map: DashMap<String, Arc<User>>,
    session_map: DashMap<Uuid, Arc<Session>>,
    ssh_key_map: DashMap<String, Arc<SshKey>>,
    user_ssh_keys: DashMap<String, Vec<String>>,
}

impl AuthStore {
    fn spawn_writer(path: PathBuf) -> mpsc::SyncSender<StoreEvent> {
        let (tx, rx) = mpsc::sync_channel::<StoreEvent>(512);

        thread::spawn(move || {
            loop {
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path.clone())
                    .expect("Couldn't open auth.ndjson");

                for event in rx.iter() {
                    match event {
                        StoreEvent::ReopenLog => {
                            eprintln!("Reopening auth.ndjson");
                            break;
                        }
                        StoreEvent::Quit => {
                            file.flush().expect("Couldn't flush auth.ndjson on quit");
                            return;
                        },
                        _ => {
                            if let Ok(mut json) = serde_json::to_string(&event) {
                                json.push('\n');
                                file.write_all(json.as_bytes()).expect("Write to auth.ndjson failed");
                            } else {
                                eprintln!("Writing event to auth.ndjson failed: {:?}", event);
                            }
                        }
                    }
                }
            }
        });

        tx
    }

    pub fn handle_event(&self, event: StoreEvent) {
        match event {
            StoreEvent::Quit => return,
            StoreEvent::ReopenLog => return,
            StoreEvent::User(user) => {
                self.user_map.insert(user.slug.clone(), Arc::new(user));
            },
            StoreEvent::UserDelete { slug } => {
                self.user_map.remove(&slug);
            },
            StoreEvent::Session(session) => {
                self.session_map.insert(session.session_id.clone(), Arc::new(session));
            },
            StoreEvent::SessionDelete { session_id } => {
                self.session_map.remove(&session_id);
            },
            StoreEvent::SshKey(ssh_key) => {
                let public_key = ssh_key.public_key.clone();
                let user_slug = ssh_key.user_slug.clone();
                self.ssh_key_map.insert(public_key.clone(), Arc::new(ssh_key));
                self.user_ssh_keys
                    .entry(user_slug)
                    .or_default()
                    .push(public_key);
            },
            StoreEvent::SshKeyDelete { public_key } => {
                if let Some((_, ssh_key)) = self.ssh_key_map.remove(&public_key) {
                    if let Some(mut keys) = self.user_ssh_keys.get_mut(&ssh_key.user_slug) {
                        keys.retain(|k| k != &public_key);
                    }
                }
            }
        }
    }

    pub(crate) fn store_event(&self, event: StoreEvent) -> Result<(), SendError<StoreEvent>>  {
        self.chan.send(event.clone())?;
        self.handle_event(event);
        Ok(())
    }

    fn replay_log(&self, path: PathBuf) {
        if let Ok(file) = File::open(path) {
            BufReader::new(file).lines()
                .flatten()
                .flat_map(|line| serde_json::from_str::<StoreEvent>(&line))
                .for_each(|event| self.handle_event(event));
        } else {
            eprintln!("No auth.ndjson found for replay, starting from scratch");
        }
    }

    pub fn new(dir_root: PathBuf) -> Self {
        let path = dir_root.join("auth.ndjson");
        let chan = Self::spawn_writer(path.clone());

        let ret = Self {
            chan,
            user_map: DashMap::new(),
            session_map: DashMap::new(),
            ssh_key_map: DashMap::new(),
            user_ssh_keys: DashMap::new(),
        };
        ret.replay_log(path);
        ret
    }

    pub fn get_user(&self, slug: &str) -> Option<Arc<User>> {
        self.user_map.get(slug).map(|v| v.value().clone())
    }

    pub fn login_user(&self, slug: &str, password: &str) -> Option<Arc<User>> {
        if let Some(user) = self.get_user(slug) {
            match user.verify_password_hash(password) {
                PasswordVerification::Valid => {
                    Some(user)
                },
                PasswordVerification::ValidNeedsRehash { new_hash } => {
                    let mut new_user = user.as_ref().clone();
                    new_user.password_hash = new_hash;
                    if new_user.save(self).is_err() {
                        None
                    } else {
                        self.get_user(slug)
                    }
                },
                PasswordVerification::Error |
                PasswordVerification::Invalid => {
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn get_user_by_session(&self, session_id: Uuid) -> Option<Arc<User>> {
        if let Some(session) = self.session_map.get(&session_id) {
            self.get_user(&session.user_slug)
        } else {
            None
        }
    }

    pub fn get_ssh_key(&self, public_key: &str) -> Option<Arc<SshKey>> {
        self.ssh_key_map.get(public_key).map(|v| v.value().clone())
    }

    pub fn get_user_by_ssh_key(&self, public_key: &str) -> Option<Arc<User>> {
        if let Some(ssh_key) = self.ssh_key_map.get(public_key) {
            self.get_user(&ssh_key.user_slug)
        } else {
            None
        }
    }

    pub fn get_ssh_keys_for_user(&self, user_slug: &str) -> Vec<Arc<SshKey>> {
        self.user_ssh_keys
            .get(user_slug)
            .map(|keys| {
                keys.iter()
                    .filter_map(|pk| self.ssh_key_map.get(pk).map(|v| v.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Drop for AuthStore {
    fn drop(&mut self) {
        self.chan.send(StoreEvent::Quit).expect("Couldn't send quit to auth.ndjson writer thread");
    }
}
