use std::sync::mpsc::SendError;

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{event::StoreEvent, AuthStore};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub slug: String,
    pub name: String,
    pub email: String,
    pub password_hash: String,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    #[serde(default)]
    pub default_main_branch: String,
    #[serde(default)]
    pub ssh_keys: Vec<String>,
}

pub enum PasswordVerification {
    Valid,
    ValidNeedsRehash { new_hash: String },
    Invalid,
    Error,
}

impl User {
    pub fn new(slug: String, name: String, email: String, password: String) -> Result<Self, argon2::password_hash::Error> {
        let password_hash = hash_password(&password)?;

        Ok(Self {
            slug,
            name,
            email,
            password_hash,
            created_at: OffsetDateTime::now_utc(),
            default_main_branch: "main".to_string(),
            ssh_keys: vec![],
        })
    }

    pub fn save(self, store: &AuthStore) -> Result<(), SendError<StoreEvent>> {
        store.store_event(StoreEvent::User(self))
    }

    pub(crate) fn verify_password_hash(&self, password: &str) -> PasswordVerification {
        let stored = &self.password_hash;
        let parsed = match PasswordHash::new(stored) {
            Ok(hash) => hash,
            Err(_) => return PasswordVerification::Error,
        };

        let hasher = password_hasher();
        if hasher
            .verify_password(password.as_bytes(), &parsed)
            .is_err()
        {
            return PasswordVerification::Invalid;
        }

        // If algorithm, version, or params differ, request a rehash for forward upgrades.
        let needs_rehash = match Params::try_from(&parsed) {
            Ok(params) => {
                let ident = argon2::password_hash::Ident::new("argon2id")
                    .expect("Couldn't initialize argon2id hasher");
                let desired_version: u32 = Version::V0x13.into();

                parsed.algorithm != ident
                    || parsed.version.unwrap_or(desired_version) != desired_version
                    || params != desired_params()
            }
            Err(_) => true,
        };

        if needs_rehash {
            match hash_password(password) {
                Ok(new_hash) => PasswordVerification::ValidNeedsRehash { new_hash },
                Err(_) => PasswordVerification::Valid, // Do not fail login if rehashing fails
            }
        } else {
            PasswordVerification::Valid
        }
    }
}

fn desired_params() -> Params {
    if cfg!(debug_assertions) {
        // Much less secure params when running a debug binary, mainly because testing is super slow otherwise
        Params::new(4 * 1024, 1, 1, None).expect("argon2 params are valid")
    } else {
        // 64 MiB memory, 4 iterations, 1 lane keeps CPU modest while resisting GPU attacks.
        Params::new(64 * 1024, 4, 1, None).expect("argon2 params are valid")
    }
}

fn password_hasher() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, desired_params())
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    password_hasher()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}
