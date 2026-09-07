use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version};
use chrono::Utc;
use language_core::{AppError, BuiltinFunction, Value};
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
    )
}

pub(crate) fn eval(function: BuiltinFunction, stack: &mut Vec<Value>) -> Result<Value, AppError> {
    match function {
        BuiltinFunction::PasswordHash => hash_password(pop_string(stack)?),
        BuiltinFunction::PasswordVerify => verify_from_stack(stack),
        BuiltinFunction::NewSessionToken
        | BuiltinFunction::NewPasswordResetToken
        | BuiltinFunction::NewCsrfToken => Ok(Value::String(generate_token())),
        BuiltinFunction::TokenHash | BuiltinFunction::PresentedTokenHash => {
            Ok(Value::String(hash_token(&pop_string(stack)?)))
        }
        BuiltinFunction::TokenMatches => token_matches_from_stack(stack),
        BuiltinFunction::TokenActive => token_active_from_stack(stack),
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
    Ok(Value::Bool(constant_time_token_eq(
        &stored_hash,
        &presented_hash,
    )))
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
    Ok(Value::Bool(constant_time_token_eq(
        &stored_hash,
        &presented_hash,
    )))
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    encode_hex(&digest)
}

fn constant_time_token_eq(stored: &str, presented: &str) -> bool {
    stored.len() == presented.len() && stored.as_bytes().ct_eq(presented.as_bytes()).into()
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
    (stack.len() >= needed)
        .then_some(())
        .ok_or(AppError::Internal)
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
        let Value::String(first) = eval(BuiltinFunction::NewSessionToken, &mut stack).unwrap()
        else {
            panic!()
        };
        let Value::String(second) = eval(BuiltinFunction::NewSessionToken, &mut stack).unwrap()
        else {
            panic!()
        };
        assert_eq!(first.len(), TOKEN_HEX_BYTES);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, second);
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
}
