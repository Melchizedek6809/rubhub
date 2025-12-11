use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

fn desired_params() -> Params {
    // 64 MiB memory, 4 iterations, 1 lane keeps CPU modest while resisting GPU attacks.
    Params::new(64 * 1024, 4, 1, None).expect("argon2 params are valid")
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
