use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng as _;
use sha2::{Digest as _, Sha256};

const DEVICE_TOKEN_PREFIX: &str = "lt_dev_";

/// Generates the only plaintext form of a Device credential.
pub fn generate_device_token() -> String {
    let mut random_bytes = [0_u8; 32];
    rand::rng().fill(&mut random_bytes);
    format!(
        "{DEVICE_TOKEN_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(random_bytes)
    )
}

/// Produces the database-safe identity of a Device credential.
pub fn digest_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}
