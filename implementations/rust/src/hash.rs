//! Domain-separated SHA-256 over Northstar JCS. Exact `sha256:` + 64 lowercase hex.

use crate::error::{Error, Result};
use crate::jcs::{canonicalize, canonicalize_json_text, Canonical, Value};
use sha2::{Digest, Sha256};

pub const HASH_PATTERN: &str = r"^sha256:[0-9a-f]{64}$";

pub fn domain_prefix(name: &str) -> Result<&'static [u8]> {
    Ok(match name {
        "intent" => b"northstar:intent:v1\0",
        "authorized-action" => b"northstar:authorized-action:v1\0",
        "executed-action" => b"northstar:executed-action:v1\0",
        "approval-context" => b"northstar:approval-context:v1\0",
        _ => return Err(Error::hash(format!("unknown domain {name}"))),
    })
}

pub fn assert_hash_string(value: &str) -> Result<()> {
    if value.len() != 71 || !value.starts_with("sha256:") {
        return Err(Error::hash(
            "expected sha256: plus 64 lowercase hex characters",
        ));
    }
    if !value[7..]
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(Error::hash(
            "expected sha256: plus 64 lowercase hex characters",
        ));
    }
    Ok(())
}

pub fn digest_hex(domain: &str, canonical: &Canonical) -> Result<String> {
    let mut h = Sha256::new();
    h.update(domain_prefix(domain)?);
    h.update(canonical.as_str().as_bytes());
    Ok(format!("{:x}", h.finalize()))
}

pub fn hash_canonical(domain: &str, canonical: &Canonical) -> Result<String> {
    Ok(format!("sha256:{}", digest_hex(domain, canonical)?))
}

pub fn hash_value(domain: &str, value: &Value) -> Result<String> {
    hash_canonical(domain, &canonicalize(value)?)
}

pub fn hash_json_text(domain: &str, text: &str) -> Result<String> {
    hash_canonical(domain, &canonicalize_json_text(text)?)
}

pub fn intent_hash(value: &Value) -> Result<String> {
    hash_value("intent", value)
}

pub fn authorized_action_hash(value: &Value) -> Result<String> {
    hash_value("authorized-action", value)
}

pub fn executed_action_hash(value: &Value) -> Result<String> {
    hash_value("executed-action", value)
}

pub fn approval_context_hash(value: &Value) -> Result<String> {
    hash_value("approval-context", value)
}
