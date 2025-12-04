use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use sea_orm::{
    ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set,
};
use uuid::Uuid;

use crate::entities::ssh_key;


pub async fn replace_ssh_keys(
    txn: &DatabaseTransaction,
    user_id: Uuid,
    ssh_keys: &[String],
) -> Result<(), sea_orm::DbErr> {
    ssh_key::Entity::delete_many()
        .filter(ssh_key::Column::UserId.eq(user_id))
        .exec(txn)
        .await?;

    if ssh_keys.is_empty() {
        return Ok(());
    }

    let mut models: Vec<ssh_key::ActiveModel> = Vec::new();
    for raw in ssh_keys {
        if let Some((public_key, hostname)) = parse_ssh_public_key(raw) {
            models.push(ssh_key::ActiveModel {
                public_key: Set(public_key),
                user_id: Set(user_id),
                hostname: Set(hostname.unwrap_or_default()),
                created_at: Set(None),
            });
        }
    }

    ssh_key::Entity::insert_many(models).exec(txn).await?;

    Ok(())
}

fn parse_ssh_public_key(input: &str) -> Option<(String, Option<String>)> {
    let mut parts = input.split_whitespace();
    let algo = parts.next()?;
    let key = parts.next()?;

    let public_key = format!("{algo} {key}");
    let hostname: Option<String> = match parts.next() {
        Some(host) => {
            let mut rest = vec![host.to_owned()];
            rest.extend(parts.map(ToOwned::to_owned));
            Some(rest.join(" "))
        }
        None => None,
    };

    Some((public_key, hostname))
}

fn desired_params() -> Params {
    // 32 MiB memory, 2 iterations, 1 lane keeps CPU modest while resisting GPU attacks.
    Params::new(32 * 1024, 2, 1, None).expect("argon2 params are valid")
}

fn password_hasher() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, desired_params())
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    password_hasher()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

pub enum PasswordVerification {
    Valid,
    ValidNeedsRehash { new_hash: String },
    Invalid,
    Error,
}

pub fn verify_password_hash(password: &str, stored: &str) -> PasswordVerification {
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
            let ident = argon2::password_hash::Ident::new("argon2id").unwrap();
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
