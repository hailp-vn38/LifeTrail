use lifetrail_server::auth::{digest_token, generate_device_token};

#[test]
fn generated_device_token_is_256_bits_base64url_and_digest_is_stable() {
    let token = generate_device_token();

    assert!(token.starts_with("lt_dev_"));
    assert_eq!(token.len(), "lt_dev_".len() + 43);
    assert!(
        token["lt_dev_".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    );
    assert_eq!(digest_token("known-token"), digest_token("known-token"));
    assert_ne!(digest_token("known-token"), digest_token("another-token"));
}
