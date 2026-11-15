//! Public-key and signature encoding support.
//!
//! Release automation may write keys as raw 32-byte values, base64, hex or a
//! PKCS#8 SubjectPublicKeyInfo PEM.  Signatures are either a raw 64-byte file
//! or base64/hex text.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::VerifyingKey;

use crate::{Result, UpdaterError};

/// DER prefix of an Ed25519 SubjectPublicKeyInfo: SEQUENCE(42) {
///   SEQUENCE(5) { OID 1.3.101.112 }, BIT STRING(33) { 0x00, key[32] }
/// }
const ED25519_SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

pub fn parse_public_key(value: &str) -> Result<VerifyingKey> {
    let text = value.trim();
    if text.is_empty() {
        return Err(UpdaterError::signature("更新公钥为空"));
    }

    let raw = if text.starts_with("-----BEGIN") {
        decode_pem(text)?
    } else {
        decode_base64_or_hex(text, 32)?
    };

    let bytes: [u8; 32] = raw
        .try_into()
        .map_err(|_| UpdaterError::signature("Ed25519 公钥长度无效"))?;
    VerifyingKey::from_bytes(&bytes)
        .map_err(|error| UpdaterError::signature(format!("Ed25519 公钥无效：{error}")))
}

fn decode_pem(text: &str) -> Result<Vec<u8>> {
    let body: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("-----"))
        .map(str::trim)
        .collect();
    let der = STANDARD
        .decode(body.as_bytes())
        .map_err(|error| UpdaterError::signature(format!("PEM 公钥不是有效 base64：{error}")))?;
    if der.len() == 32 {
        return Ok(der);
    }
    if der.len() == 44 && der.starts_with(&ED25519_SPKI_PREFIX) {
        return Ok(der[12..44].to_vec());
    }
    Err(UpdaterError::signature("PEM 公钥不是 Ed25519 SPKI"))
}

/// Decode base64 or hex text into exactly `expected` bytes.
///
/// Base64 is tried first so the historical base64 key format keeps working;
/// hex is tried when the base64 result has the wrong length.  This is slightly
/// more permissive than the Python implementation, which stops after a
/// successful base64 decode even when the length is wrong, but it accepts the
/// documented hex format reliably.
pub fn decode_base64_or_hex(text: &str, expected: usize) -> Result<Vec<u8>> {
    let text = text.trim();
    if text.is_empty() {
        return Err(UpdaterError::signature("签名或公钥数据为空"));
    }
    if let Ok(decoded) = STANDARD.decode(text.as_bytes()) {
        if decoded.len() == expected {
            return Ok(decoded);
        }
    }
    if text.len() == expected * 2 && text.chars().all(|character| character.is_ascii_hexdigit()) {
        let mut decoded = Vec::with_capacity(expected);
        for index in (0..text.len()).step_by(2) {
            let byte = u8::from_str_radix(&text[index..index + 2], 16)
                .map_err(|error| UpdaterError::signature(format!("十六进制数据无效：{error}")))?;
            decoded.push(byte);
        }
        return Ok(decoded);
    }
    Err(UpdaterError::signature(format!(
        "数据不是有效的 base64 或十六进制（期望 {expected} 字节）"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn public_key_bytes() -> [u8; 32] {
        SigningKey::from_bytes(&[7_u8; 32])
            .verifying_key()
            .to_bytes()
    }

    #[test]
    fn raw_base64_and_hex_keys_are_accepted() {
        let bytes = public_key_bytes();
        let base64 = STANDARD.encode(bytes);
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();

        assert_eq!(parse_public_key(&base64).unwrap().to_bytes(), bytes);
        assert_eq!(parse_public_key(&hex).unwrap().to_bytes(), bytes);
    }

    #[test]
    fn pem_public_key_is_accepted() {
        let bytes = public_key_bytes();
        let mut der = ED25519_SPKI_PREFIX.to_vec();
        der.extend_from_slice(&bytes);
        let body = STANDARD.encode(der);
        let pem = format!("-----BEGIN PUBLIC KEY-----\n{body}\n-----END PUBLIC KEY-----\n");

        assert_eq!(parse_public_key(&pem).unwrap().to_bytes(), bytes);
    }

    #[test]
    fn invalid_keys_are_rejected() {
        assert!(parse_public_key("").is_err());
        assert!(parse_public_key("not-a-key").is_err());
        assert!(parse_public_key(&STANDARD.encode([1_u8; 31])).is_err());
        assert!(
            parse_public_key("-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----").is_err()
        );
    }
}
