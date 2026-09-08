use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version};
use chrono::Utc;
use language_core::{AppError, BuiltinFunction, Value};
use ring::aead::{self, Aad, LessSafeKey, Nonce, UnboundKey};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

const PASSWORD_MIN_BYTES: usize = 12;
const PASSWORD_MAX_BYTES: usize = 1024;
const TOKEN_BYTES: usize = 32;
const TOKEN_HEX_BYTES: usize = TOKEN_BYTES * 2;

pub(crate) fn handles(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::PasswordHash
            | BuiltinFunction::PasswordVerify
            | BuiltinFunction::NewSessionToken
            | BuiltinFunction::NewPasswordResetToken
            | BuiltinFunction::NewCsrfToken
            | BuiltinFunction::TokenHash
            | BuiltinFunction::PresentedTokenHash
            | BuiltinFunction::TokenMatches
            | BuiltinFunction::TokenActive
            | BuiltinFunction::SignWebhook
            | BuiltinFunction::VerifyWebhookSignature
            | BuiltinFunction::EncryptUserData
            | BuiltinFunction::DecryptUserData
    )
}

pub(crate) fn eval(function: BuiltinFunction, stack: &mut Vec<Value>) -> Result<Value, AppError> {
    match function {
        BuiltinFunction::PasswordHash => hash_password(pop_string(stack)?),
        BuiltinFunction::PasswordVerify => verify_from_stack(stack),
        BuiltinFunction::NewSessionToken
        | BuiltinFunction::NewPasswordResetToken
        | BuiltinFunction::NewCsrfToken => Ok(Value::String(generate_token())),
        BuiltinFunction::TokenHash | BuiltinFunction::PresentedTokenHash => Ok(Value::String(hash_token(&pop_string(stack)?))),
        BuiltinFunction::TokenMatches => token_matches_from_stack(stack),
        BuiltinFunction::TokenActive => token_active_from_stack(stack),
        BuiltinFunction::SignWebhook => sign_webhook_from_stack(stack),
        BuiltinFunction::VerifyWebhookSignature => verify_webhook_from_stack(stack),
        BuiltinFunction::EncryptUserData => encrypt_user_data_from_stack(stack),
        BuiltinFunction::DecryptUserData => decrypt_user_data_from_stack(stack),
        _ => Err(AppError::Internal),
    }
}

pub(crate) fn estimated_result_alloc(
    function: BuiltinFunction,
    stack: &[Value],
) -> Result<u64, AppError> {
    match function {
        BuiltinFunction::PasswordHash => Ok(256),
        BuiltinFunction::PasswordVerify | BuiltinFunction::TokenMatches => {
            require_stack_len(stack, 2)?;
            Ok(0)
        }
        BuiltinFunction::TokenActive => {
            require_stack_len(stack, 3)?;
            Ok(0)
        }
        BuiltinFunction::TokenHash | BuiltinFunction::PresentedTokenHash => {
            require_stack_len(stack, 1)?;
            Ok(TOKEN_HEX_BYTES as u64)
        }
        BuiltinFunction::NewSessionToken
        | BuiltinFunction::NewPasswordResetToken
        | BuiltinFunction::NewCsrfToken => Ok(TOKEN_HEX_BYTES as u64),
        BuiltinFunction::SignWebhook => {
            require_stack_len(stack, 2)?;
            Ok(TOKEN_HEX_BYTES as u64)
        }
        BuiltinFunction::VerifyWebhookSignature => {
            require_stack_len(stack, 3)?;
            Ok(0)
        }
        BuiltinFunction::EncryptUserData => {
            require_stack_len(stack, 2)?;
            let plaintext = stack.last().and_then(|v| match v { Value::String(s) => Some(s.len()), _ => None }).ok_or(AppError::Internal)?;
            Ok((plaintext.saturating_add(16) * 2 + 40) as u64)
        }
        BuiltinFunction::DecryptUserData => {
            require_stack_len(stack, 2)?;
            Ok(stack.last().and_then(|v| match v { Value::String(s) => Some(s.len()), _ => None }).unwrap_or(0) as u64)
        }
        _ => Err(AppError::Internal),
    }
}

fn hash_password(password: String) -> Result<Value, AppError> {
    if !(PASSWORD_MIN_BYTES..=PASSWORD_MAX_BYTES).contains(&password.len()) {
        return Err(AppError::BadRequest);
    }
    let mut salt_bytes = [0u8; 16];
    rand::fill(&mut salt_bytes[..]);
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| AppError::Internal)?;
    let encoded = argon2_instance()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AppError::Internal)?
        .to_string();
    Ok(Value::String(encoded))
}

fn verify_from_stack(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let password = pop_string(stack)?;
    let encoded = pop_string(stack)?;
    Ok(Value::Bool(verify_password(&encoded, &password)))
}

fn verify_password(encoded: &str, password: &str) -> bool {
    PasswordHash::new(encoded).ok().is_some_and(|parsed| {
        argon2_instance()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::fill(&mut bytes[..]);
    encode_hex(&bytes)
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn token_matches_from_stack(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let presented = pop_string(stack)?;
    let stored_hash = pop_string(stack)?;
    let presented_hash = hash_token(&presented);
    Ok(Value::Bool(constant_time_token_eq(&stored_hash, &presented_hash)))
}

fn token_active_from_stack(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let expires_at = match stack.pop().ok_or(AppError::Internal)? {
        Value::DateTime(value) => value,
        _ => return Err(AppError::Internal),
    };
    let presented = pop_string(stack)?;
    let stored_hash = pop_string(stack)?;
    if expires_at <= Utc::now() {
        return Ok(Value::Bool(false));
    }
    let presented_hash = hash_token(&presented);
    Ok(Value::Bool(constant_time_token_eq(&stored_hash, &presented_hash)))
}

fn sign_webhook_from_stack(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let payload = pop_string(stack)?;
    let key = pop_string(stack)?;
    if !(32..=4096).contains(&key.len()) {
        return Err(AppError::Internal);
    }
    Ok(Value::String(hmac_sha256_hex(key.as_bytes(), payload.as_bytes())))
}

fn verify_webhook_from_stack(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let signature = pop_string(stack)?;
    let payload = pop_string(stack)?;
    let key = pop_string(stack)?;
    if !(32..=4096).contains(&key.len()) {
        return Err(AppError::Internal);
    }
    let expected = hmac_sha256_hex(key.as_bytes(), payload.as_bytes());
    Ok(Value::Bool(constant_time_token_eq(&expected, &signature)))
}

fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut key_block = [0u8; BLOCK];
    if key.len() > BLOCK {
        let digest = Sha256::digest(key);
        key_block[..digest.len()].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    encode_hex(&outer.finalize())
}


const USER_DATA_PLAINTEXT_MAX_BYTES: usize = 1024 * 1024;
const USER_DATA_ENVELOPE_PREFIX: &str = "rwenc1";
const USER_DATA_KEY_HEX_BYTES: usize = 64;
const USER_DATA_NONCE_BYTES: usize = 12;

fn encrypt_user_data_from_stack(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let plaintext = pop_string(stack)?;
    let key = pop_string(stack)?;
    if plaintext.len() > USER_DATA_PLAINTEXT_MAX_BYTES {
        return Err(AppError::BadRequest);
    }
    let key_bytes = decode_fixed_hex::<32>(&key).ok_or(AppError::Internal)?;
    let unbound = UnboundKey::new(&aead::AES_256_GCM, &key_bytes).map_err(|_| AppError::Internal)?;
    let key = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; USER_DATA_NONCE_BYTES];
    rand::fill(&mut nonce_bytes[..]);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plaintext.into_bytes();
    key.seal_in_place_append_tag(nonce, Aad::from(USER_DATA_ENVELOPE_PREFIX.as_bytes()), &mut in_out)
        .map_err(|_| AppError::Internal)?;
    Ok(Value::String(format!(
        "{}:{}:{}",
        USER_DATA_ENVELOPE_PREFIX,
        encode_hex(&nonce_bytes),
        encode_hex(&in_out)
    )))
}

fn decrypt_user_data_from_stack(stack: &mut Vec<Value>) -> Result<Value, AppError> {
    let envelope = pop_string(stack)?;
    let key = pop_string(stack)?;
    let key_bytes = decode_fixed_hex::<32>(&key).ok_or(AppError::Internal)?;
    let mut parts = envelope.split(':');
    if parts.next() != Some(USER_DATA_ENVELOPE_PREFIX) {
        return Err(AppError::Internal);
    }
    let nonce_hex = parts.next().ok_or(AppError::Internal)?;
    let ciphertext_hex = parts.next().ok_or(AppError::Internal)?;
    if parts.next().is_some() || nonce_hex.len() != USER_DATA_NONCE_BYTES * 2 {
        return Err(AppError::Internal);
    }
    let nonce_bytes = decode_fixed_hex::<USER_DATA_NONCE_BYTES>(nonce_hex).ok_or(AppError::Internal)?;
    let mut ciphertext = decode_hex(ciphertext_hex).ok_or(AppError::Internal)?;
    if ciphertext.len() < aead::AES_256_GCM.tag_len()
        || ciphertext.len() > USER_DATA_PLAINTEXT_MAX_BYTES + aead::AES_256_GCM.tag_len()
    {
        return Err(AppError::Internal);
    }
    let unbound = UnboundKey::new(&aead::AES_256_GCM, &key_bytes).map_err(|_| AppError::Internal)?;
    let key = LessSafeKey::new(unbound);
    let plaintext = key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce_bytes),
            Aad::from(USER_DATA_ENVELOPE_PREFIX.as_bytes()),
            &mut ciphertext,
        )
        .map_err(|_| AppError::Internal)?;
    let plaintext = String::from_utf8(plaintext.to_vec()).map_err(|_| AppError::Internal)?;
    Ok(Value::String(plaintext))
}

fn decode_fixed_hex<const N: usize>(raw: &str) -> Option<[u8; N]> {
    if raw.len() != N * 2 || (N == 32 && raw.len() != USER_DATA_KEY_HEX_BYTES) {
        return None;
    }
    let bytes = decode_hex(raw)?;
    bytes.try_into().ok()
}

fn decode_hex(raw: &str) -> Option<Vec<u8>> {
    if raw.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(raw.len() / 2);
    let bytes = raw.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let hi = hex_nibble(pair[0])?;
        let lo = hex_nibble(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    encode_hex(&digest)
}

fn constant_time_token_eq(stored: &str, presented: &str) -> bool {
    stored.len() == presented.len()
        && stored.as_bytes().ct_eq(presented.as_bytes()).into()
}

fn argon2_instance() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19 * 1024, 2, 1, None).expect("valid Argon2id params"),
    )
}

fn pop_string(stack: &mut Vec<Value>) -> Result<String, AppError> {
    let Value::String(value) = stack.pop().ok_or(AppError::Internal)? else {
        return Err(AppError::Internal);
    };
    Ok(value)
}

fn require_stack_len(stack: &[Value], needed: usize) -> Result<(), AppError> {
    (stack.len() >= needed).then_some(()).ok_or(AppError::Internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_roundtrip() {
        let mut stack = vec![Value::String("correct horse battery staple".into())];
        let Value::String(hash) = eval(BuiltinFunction::PasswordHash, &mut stack).unwrap() else {
            panic!()
        };
        let mut verify = vec![
            Value::String(hash),
            Value::String("correct horse battery staple".into()),
        ];
        assert_eq!(
            eval(BuiltinFunction::PasswordVerify, &mut verify).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn issued_tokens_are_256_bit_hex_values() {
        let mut stack = Vec::new();
        let Value::String(first) = eval(BuiltinFunction::NewSessionToken, &mut stack).unwrap() else {
            panic!()
        };
        let Value::String(second) = eval(BuiltinFunction::NewSessionToken, &mut stack).unwrap() else {
            panic!()
        };
        assert_eq!(first.len(), TOKEN_HEX_BYTES);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn user_data_aead_roundtrip_and_nonce_uniqueness() {
        let key = "11".repeat(32);
        let plaintext = "classified user profile".to_string();
        let mut first_stack = vec![Value::String(key.clone()), Value::String(plaintext.clone())];
        let Value::String(first) = eval(BuiltinFunction::EncryptUserData, &mut first_stack).unwrap() else {
            panic!()
        };
        let mut second_stack = vec![Value::String(key.clone()), Value::String(plaintext.clone())];
        let Value::String(second) = eval(BuiltinFunction::EncryptUserData, &mut second_stack).unwrap() else {
            panic!()
        };
        assert!(first.starts_with("rwenc1:"));
        assert_ne!(first, second, "platform-generated nonces must make envelopes unique");
        let mut decrypt_stack = vec![Value::String(key), Value::String(first)];
        assert_eq!(
            eval(BuiltinFunction::DecryptUserData, &mut decrypt_stack).unwrap(),
            Value::String(plaintext)
        );
    }

    #[test]
    fn user_data_aead_rejects_tampering_and_wrong_key() {
        let key = "22".repeat(32);
        let mut encrypt_stack = vec![Value::String(key.clone()), Value::String("secret".into())];
        let Value::String(envelope) = eval(BuiltinFunction::EncryptUserData, &mut encrypt_stack).unwrap() else {
            panic!()
        };
        let mut tampered = envelope.clone().into_bytes();
        let last = tampered.len() - 1;
        tampered[last] = if tampered[last] == b'0' { b'1' } else { b'0' };
        let tampered = String::from_utf8(tampered).unwrap();
        let mut tampered_stack = vec![Value::String(key), Value::String(tampered)];
        assert!(eval(BuiltinFunction::DecryptUserData, &mut tampered_stack).is_err());

        let mut wrong_key_stack = vec![Value::String("33".repeat(32)), Value::String(envelope)];
        assert!(eval(BuiltinFunction::DecryptUserData, &mut wrong_key_stack).is_err());
    }

    #[test]
    fn token_hash_is_fixed_size_and_token_matches_hashes_presented_value() {
        let token = generate_token();
        let hash = hash_token(&token);
        assert_eq!(hash.len(), TOKEN_HEX_BYTES);
        let mut matching = vec![Value::String(hash.clone()), Value::String(token.clone())];
        assert_eq!(
            eval(BuiltinFunction::TokenMatches, &mut matching).unwrap(),
            Value::Bool(true)
        );
        let mut different = vec![Value::String(hash), Value::String(generate_token())];
        assert_eq!(
            eval(BuiltinFunction::TokenMatches, &mut different).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn token_active_requires_matching_hash_and_future_expiry() {
        let token = generate_token();
        let hash = hash_token(&token);
        let mut active = vec![
            Value::String(hash.clone()),
            Value::String(token.clone()),
            Value::DateTime(Utc::now() + chrono::Duration::minutes(5)),
        ];
        assert_eq!(
            eval(BuiltinFunction::TokenActive, &mut active).unwrap(),
            Value::Bool(true)
        );

        let mut expired = vec![
            Value::String(hash),
            Value::String(token),
            Value::DateTime(Utc::now() - chrono::Duration::seconds(1)),
        ];
        assert_eq!(
            eval(BuiltinFunction::TokenActive, &mut expired).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn webhook_signatures_roundtrip_and_reject_changes() {
        let key = "k".repeat(32);
        let payload = "event=paid".to_string();
        let mut sign = vec![Value::String(key.clone()), Value::String(payload.clone())];
        let Value::String(signature) = eval(BuiltinFunction::SignWebhook, &mut sign).unwrap() else { panic!() };
        let mut verify = vec![Value::String(key.clone()), Value::String(payload), Value::String(signature.clone())];
        assert_eq!(eval(BuiltinFunction::VerifyWebhookSignature, &mut verify).unwrap(), Value::Bool(true));
        let mut changed = vec![Value::String(key), Value::String("event=refunded".into()), Value::String(signature)];
        assert_eq!(eval(BuiltinFunction::VerifyWebhookSignature, &mut changed).unwrap(), Value::Bool(false));
    }

}
