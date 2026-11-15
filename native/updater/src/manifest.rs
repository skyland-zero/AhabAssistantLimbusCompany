//! Ed25519 verification for downloaded release archives.
//!
//! The signed message is the exact manifest byte stream on disk.  The updater
//! never re-serialises the manifest before verifying it, so canonical JSON
//! ordering stays a property of the release signer rather than of this tool.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

use ed25519_dalek::{Signature, Verifier};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::keys;
use crate::{Result, UpdaterError};

const REQUIRED_FIELDS: [&str; 6] = [
    "schemaVersion",
    "version",
    "artifact",
    "size",
    "sha256",
    "keyId",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedManifest {
    pub version: String,
    pub artifact: String,
    pub size: u64,
    pub sha256: String,
    pub key_id: String,
}

struct ManifestFields {
    version: String,
    artifact: String,
    size: u64,
    sha256: String,
    key_id: String,
}

pub fn verify_files(
    manifest_path: &Path,
    signature_path: &Path,
    archive_path: &Path,
    public_keys: Option<&str>,
) -> Result<SignedManifest> {
    let manifest_bytes = fs::read(manifest_path).map_err(|error| {
        UpdaterError::signature(format!(
            "无法读取更新清单 {}：{error}",
            manifest_path.display()
        ))
    })?;
    let signature_bytes = fs::read(signature_path).map_err(|error| {
        UpdaterError::signature(format!(
            "无法读取更新签名 {}：{error}",
            signature_path.display()
        ))
    })?;
    verify_bytes(&manifest_bytes, &signature_bytes, archive_path, public_keys)
}

pub fn verify_bytes(
    manifest_bytes: &[u8],
    signature_bytes: &[u8],
    archive_path: &Path,
    public_keys: Option<&str>,
) -> Result<SignedManifest> {
    let fields = parse_manifest(manifest_bytes)?;
    let keys = public_key_mapping(public_keys)?;
    let key_text = keys.get(&fields.key_id).ok_or_else(|| {
        UpdaterError::signature(format!("更新清单引用了未知公钥：{}", fields.key_id))
    })?;
    let verifying_key = keys::parse_public_key(key_text)?;
    let signature = decode_signature(signature_bytes)?;
    verifying_key
        .verify(manifest_bytes, &signature)
        .map_err(|_| UpdaterError::signature("更新清单签名校验失败"))?;

    verify_archive(archive_path, &fields)?;

    Ok(SignedManifest {
        version: fields.version,
        artifact: fields.artifact,
        size: fields.size,
        sha256: fields.sha256,
        key_id: fields.key_id,
    })
}

fn parse_manifest(manifest_bytes: &[u8]) -> Result<ManifestFields> {
    let payload: Value = serde_json::from_slice(manifest_bytes)
        .map_err(|error| UpdaterError::manifest(format!("更新清单不是有效 JSON：{error}")))?;
    let object = payload
        .as_object()
        .ok_or_else(|| UpdaterError::manifest("更新清单必须是对象"))?;

    if object.len() != REQUIRED_FIELDS.len()
        || REQUIRED_FIELDS
            .iter()
            .any(|field| !object.contains_key(*field))
    {
        return Err(UpdaterError::manifest("更新清单字段不完整或包含未知字段"));
    }

    if object.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
        return Err(UpdaterError::manifest("更新清单协议版本不兼容"));
    }

    let version = object
        .get("version")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| UpdaterError::manifest("更新清单版本无效"))?;
    let version = version.to_string();

    let artifact = object
        .get("artifact")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| UpdaterError::manifest("更新清单 artifact 无效"))?;
    if artifact == "."
        || artifact == ".."
        || artifact
            .chars()
            .any(|character| matches!(character, '/' | '\\' | ':'))
        || Path::new(artifact)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(artifact)
    {
        return Err(UpdaterError::manifest("更新清单 artifact 必须是文件名"));
    }
    let artifact = artifact.to_string();

    let size = object
        .get("size")
        .and_then(Value::as_u64)
        .ok_or_else(|| UpdaterError::manifest("更新清单文件大小无效"))?;

    let sha256 = object
        .get("sha256")
        .and_then(Value::as_str)
        .filter(|value| value.len() == 64)
        .filter(|value| value.chars().all(|character| character.is_ascii_hexdigit()))
        .ok_or_else(|| UpdaterError::manifest("更新清单 SHA-256 无效"))?
        .to_lowercase();

    let key_id = object
        .get("keyId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| UpdaterError::manifest("更新清单 keyId 无效"))?
        .to_string();

    Ok(ManifestFields {
        version,
        artifact,
        size,
        sha256,
        key_id,
    })
}

fn decode_signature(signature_bytes: &[u8]) -> Result<Signature> {
    let raw = if signature_bytes.len() == 64 {
        signature_bytes.to_vec()
    } else {
        let text = std::str::from_utf8(signature_bytes)
            .map_err(|_| UpdaterError::signature("更新签名不是有效文本"))?;
        keys::decode_base64_or_hex(text, 64)?
    };
    Signature::from_slice(&raw).map_err(|_| UpdaterError::signature("Ed25519 签名长度无效"))
}

fn public_key_mapping(public_keys: Option<&str>) -> Result<BTreeMap<String, String>> {
    let configured = public_keys
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            env::var("AALC_UPDATE_PUBLIC_KEYS")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .or_else(|| {
            env::var("AALC_UPDATE_PUBLIC_KEY")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .ok_or_else(|| UpdaterError::signature("未配置更新公钥"))?;

    if configured.starts_with('{') {
        let parsed: Value = serde_json::from_str(&configured).map_err(|error| {
            UpdaterError::signature(format!("更新公钥映射不是有效 JSON：{error}"))
        })?;
        let object: Map<String, Value> = parsed
            .as_object()
            .cloned()
            .ok_or_else(|| UpdaterError::signature("更新公钥映射必须是对象"))?;
        let mut mapping = BTreeMap::new();
        for (key_id, value) in object {
            let value = value
                .as_str()
                .ok_or_else(|| UpdaterError::signature("更新公钥值必须是字符串"))?;
            mapping.insert(key_id, value.to_string());
        }
        if mapping.is_empty() {
            return Err(UpdaterError::signature("更新公钥映射不能为空"));
        }
        return Ok(mapping);
    }

    Ok(BTreeMap::from([("primary".to_string(), configured)]))
}

fn verify_archive(archive_path: &Path, fields: &ManifestFields) -> Result<()> {
    let metadata = fs::metadata(archive_path).map_err(|error| {
        UpdaterError::signature(format!(
            "更新包不存在或无法读取 {}：{error}",
            archive_path.display()
        ))
    })?;
    if !metadata.is_file() {
        return Err(UpdaterError::signature("更新包不是文件"));
    }
    let name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| UpdaterError::signature("更新包文件名无效"))?;
    if name != fields.artifact {
        return Err(UpdaterError::signature("更新清单与下载文件名不匹配"));
    }
    if metadata.len() != fields.size {
        return Err(UpdaterError::signature("更新包大小与清单不匹配"));
    }

    let mut digest = Sha256::new();
    let mut stream = fs::File::open(archive_path)
        .map_err(|error| UpdaterError::signature(format!("无法读取更新包：{error}")))?;
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = std::io::Read::read(&mut stream, &mut buffer)
            .map_err(|error| UpdaterError::signature(format!("读取更新包失败：{error}")))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    let actual = format!("{:x}", digest.finalize());
    if actual != fields.sha256 {
        return Err(UpdaterError::signature("更新包 SHA-256 与清单不匹配"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    fn unique_dir() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aalc-updater-manifest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn signed_fixture(
        content: &[u8],
        extra: Option<(&str, Value)>,
    ) -> (std::path::PathBuf, Vec<u8>, Vec<u8>, String) {
        let directory = unique_dir();
        let archive_name = "AALC.nsh_1.2.3.7z";
        let archive_path = directory.join(archive_name);
        fs::write(&archive_path, content).unwrap();

        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());
        let mut payload = serde_json::json!({
            "artifact": archive_name,
            "keyId": "primary",
            "schemaVersion": 1,
            "sha256": format!("{:x}", Sha256::digest(content)),
            "size": content.len(),
            "version": "v1.2.3",
        });
        if let Some((key, value)) = extra {
            payload
                .as_object_mut()
                .expect("manifest object")
                .insert(key.to_string(), value);
        }
        let manifest = serde_json::to_vec(&payload).unwrap();
        let signature = signing_key.sign(&manifest).to_bytes().to_vec();
        (archive_path, manifest, signature, public_key)
    }

    fn keys_json(public_key: &str) -> String {
        format!(r#"{{"primary":"{public_key}"}}"#)
    }

    #[test]
    fn a_valid_manifest_binds_the_archive() {
        let content = b"signed archive fixture";
        let (archive, manifest, signature, public_key) = signed_fixture(content, None);

        let verified = verify_bytes(
            &manifest,
            &signature,
            &archive,
            Some(&keys_json(&public_key)),
        )
        .expect("manifest must verify");

        assert_eq!(verified.version, "v1.2.3");
        assert_eq!(verified.artifact, "AALC.nsh_1.2.3.7z");
        assert_eq!(verified.size, content.len() as u64);
        assert_eq!(verified.sha256, format!("{:x}", Sha256::digest(content)));
        assert_eq!(verified.key_id, "primary");
        let _ = fs::remove_dir_all(archive.parent().unwrap());
    }

    #[test]
    fn a_tampered_manifest_is_rejected() {
        let (archive, manifest, signature, public_key) = signed_fixture(b"archive", None);
        let tampered = String::from_utf8(manifest)
            .unwrap()
            .replace("v1.2.3", "v1.2.4")
            .into_bytes();

        let error = verify_bytes(
            &tampered,
            &signature,
            &archive,
            Some(&keys_json(&public_key)),
        )
        .expect_err("tampered manifest must fail");
        assert!(error.message.contains("签名"));
        let _ = fs::remove_dir_all(archive.parent().unwrap());
    }

    #[test]
    fn a_tampered_archive_is_rejected() {
        let (archive, manifest, signature, public_key) = signed_fixture(b"original", None);
        fs::write(&archive, b"tampered").unwrap();

        let error = verify_bytes(
            &manifest,
            &signature,
            &archive,
            Some(&keys_json(&public_key)),
        )
        .expect_err("tampered archive must fail");
        assert!(error.message.contains("大小") || error.message.contains("SHA-256"));
        let _ = fs::remove_dir_all(archive.parent().unwrap());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let content = b"archive";
        let (archive, manifest, signature, public_key) =
            signed_fixture(content, Some(("unexpected", Value::Bool(true))));

        let error = verify_bytes(
            &manifest,
            &signature,
            &archive,
            Some(&keys_json(&public_key)),
        )
        .expect_err("unknown manifest fields must fail");
        assert!(error.message.contains("字段"));
        let _ = fs::remove_dir_all(archive.parent().unwrap());
    }

    #[test]
    fn artifact_must_match_the_download_file() {
        let content = b"archive";
        let (archive, manifest, signature, public_key) = signed_fixture(content, None);
        let other = archive.parent().unwrap().join("other.7z");
        fs::write(&other, content).unwrap();

        let error = verify_bytes(&manifest, &signature, &other, Some(&keys_json(&public_key)))
            .expect_err("artifact mismatch must fail");
        assert!(error.message.contains("文件名"));
        let _ = fs::remove_dir_all(archive.parent().unwrap());
    }
}
