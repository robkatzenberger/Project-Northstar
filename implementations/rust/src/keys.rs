//! Purpose-bound local key registry for the pre-release authority profile.
//!
//! Key bytes remain deployment inputs. The registry prevents one configured
//! secret from silently serving more than one cryptographic role and retains
//! verify-only keys so rotation does not destroy historical verifiability.

use crate::error::{Error, Result};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

type HmacSha256 = Hmac<Sha256>;

const PROOF_PREFIX: &[u8] = b"northstar:role-key-proof:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyPurpose {
    AuditSealing,
    AuthorizationMac,
    ServiceIdentity,
    OperatorAuthentication,
    TenantTrust,
}

impl KeyPurpose {
    pub const ALL: [Self; 5] = [
        Self::AuditSealing,
        Self::AuthorizationMac,
        Self::ServiceIdentity,
        Self::OperatorAuthentication,
        Self::TenantTrust,
    ];

    pub const REQUIRED_LOCAL_AUTHORITY: [Self; 2] = [Self::AuditSealing, Self::AuthorizationMac];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuditSealing => "AUDIT_SEALING",
            Self::AuthorizationMac => "AUTHORIZATION_MAC",
            Self::ServiceIdentity => "SERVICE_IDENTITY",
            Self::OperatorAuthentication => "OPERATOR_AUTHENTICATION",
            Self::TenantTrust => "TENANT_TRUST",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Active,
    VerifyOnly,
    Revoked,
}

#[derive(Clone)]
pub struct RoleKey {
    pub key_id: String,
    pub purpose: KeyPurpose,
    pub state: KeyState,
    pub material: Vec<u8>,
}

impl fmt::Debug for RoleKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RoleKey")
            .field("key_id", &self.key_id)
            .field("purpose", &self.purpose)
            .field("state", &self.state)
            .field("material", &"[REDACTED]")
            .finish()
    }
}

impl RoleKey {
    pub fn active(key_id: impl Into<String>, purpose: KeyPurpose, material: Vec<u8>) -> Self {
        Self {
            key_id: key_id.into(),
            purpose,
            state: KeyState::Active,
            material,
        }
    }

    pub fn verify_only(key_id: impl Into<String>, purpose: KeyPurpose, material: Vec<u8>) -> Self {
        Self {
            key_id: key_id.into(),
            purpose,
            state: KeyState::VerifyOnly,
            material,
        }
    }

    pub fn revoked(key_id: impl Into<String>, purpose: KeyPurpose, material: Vec<u8>) -> Self {
        Self {
            key_id: key_id.into(),
            purpose,
            state: KeyState::Revoked,
            material,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyProof {
    pub key_id: String,
    pub algorithm: String,
    pub proof: String,
}

#[derive(Clone)]
pub struct KeyRing {
    keys: BTreeMap<String, RoleKey>,
    active: BTreeMap<KeyPurpose, String>,
}

impl fmt::Debug for KeyRing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyRing")
            .field("key_ids", &self.keys.keys().collect::<Vec<_>>())
            .field("active", &self.active)
            .finish()
    }
}

impl KeyRing {
    pub fn active_local_authority_profile(entries: [(String, Vec<u8>); 2]) -> Result<Self> {
        Self::new(
            KeyPurpose::REQUIRED_LOCAL_AUTHORITY
                .into_iter()
                .zip(entries)
                .map(|(purpose, (key_id, material))| RoleKey::active(key_id, purpose, material))
                .collect(),
        )
    }

    pub fn new(keys: Vec<RoleKey>) -> Result<Self> {
        let mut by_id = BTreeMap::new();
        let mut active = BTreeMap::new();
        let mut material_fingerprints = BTreeSet::new();
        for key in keys {
            validate_key_id(&key.key_id)?;
            if key.material.len() < 32 {
                return Err(key_config("every role key must contain at least 32 bytes"));
            }
            let fingerprint = Sha256::digest(&key.material).to_vec();
            if !material_fingerprints.insert(fingerprint) {
                return Err(key_config(
                    "role key material must not be reused across key ids or purposes",
                ));
            }
            if key.state == KeyState::Active
                && active.insert(key.purpose, key.key_id.clone()).is_some()
            {
                return Err(key_config(format!(
                    "multiple active keys configured for {}",
                    key.purpose.as_str()
                )));
            }
            if by_id.insert(key.key_id.clone(), key).is_some() {
                return Err(key_config("duplicate role key id"));
            }
        }
        for purpose in KeyPurpose::REQUIRED_LOCAL_AUTHORITY {
            if !active.contains_key(&purpose) {
                return Err(key_config(format!(
                    "one active {} key is required",
                    purpose.as_str()
                )));
            }
        }
        Ok(Self {
            keys: by_id,
            active,
        })
    }

    pub fn sign(&self, purpose: KeyPurpose, payload: &[u8]) -> Result<KeyProof> {
        let key_id = self
            .active
            .get(&purpose)
            .ok_or_else(|| key_config(format!("no active {} key", purpose.as_str())))?;
        let key = self.key_for_use(key_id, purpose, true)?;
        Ok(KeyProof {
            key_id: key.key_id.clone(),
            algorithm: "HMAC-SHA256".into(),
            proof: compute_proof(key, payload)?,
        })
    }

    pub fn verify(
        &self,
        key_id: &str,
        purpose: KeyPurpose,
        payload: &[u8],
        proof: &str,
    ) -> Result<()> {
        let key = self.key_for_use(key_id, purpose, false)?;
        let encoded = proof
            .strip_prefix("hmac-sha256:")
            .ok_or_else(|| key_proof("unsupported role-key proof format"))?;
        let bytes = decode_hex_32(encoded)?;
        let mac = mac(key, payload)?;
        mac.verify_slice(&bytes)
            .map_err(|_| key_proof("role-key proof verification failed"))
    }

    pub fn purpose_for(&self, key_id: &str) -> Option<KeyPurpose> {
        self.keys.get(key_id).map(|key| key.purpose)
    }

    pub fn state_for(&self, key_id: &str) -> Option<KeyState> {
        self.keys.get(key_id).map(|key| key.state)
    }

    pub fn active_key_id(&self, purpose: KeyPurpose) -> Option<&str> {
        self.active.get(&purpose).map(String::as_str)
    }

    fn key_for_use(&self, key_id: &str, purpose: KeyPurpose, signing: bool) -> Result<&RoleKey> {
        let key = self
            .keys
            .get(key_id)
            .ok_or_else(|| key_config("unknown role key id"))?;
        if key.purpose != purpose {
            return Err(Error::coded(
                "KEY_ROLE_MISMATCH",
                "role key cannot be used for a different purpose",
            ));
        }
        match key.state {
            KeyState::Active => Ok(key),
            KeyState::VerifyOnly if !signing => Ok(key),
            KeyState::VerifyOnly => Err(Error::coded(
                "KEY_NOT_ACTIVE",
                "verify-only key cannot create new proofs",
            )),
            KeyState::Revoked => Err(Error::coded(
                "KEY_REVOKED",
                "revoked key cannot create or verify proofs",
            )),
        }
    }
}

fn compute_proof(key: &RoleKey, payload: &[u8]) -> Result<String> {
    let mac = mac(key, payload)?;
    Ok(format!("hmac-sha256:{}", hex(&mac.finalize().into_bytes())))
}

fn mac(key: &RoleKey, payload: &[u8]) -> Result<HmacSha256> {
    let mut mac = HmacSha256::new_from_slice(&key.material)
        .map_err(|_| key_config("invalid role key material"))?;
    mac.update(PROOF_PREFIX);
    mac.update(key.purpose.as_str().as_bytes());
    mac.update(&[0]);
    mac.update(key.key_id.as_bytes());
    mac.update(&[0]);
    mac.update(payload);
    Ok(mac)
}

fn validate_key_id(key_id: &str) -> Result<()> {
    if key_id.is_empty()
        || key_id.len() > 128
        || !key_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(key_config("role key id is invalid"));
    }
    Ok(())
}

fn decode_hex_32(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64 {
        return Err(key_proof("role-key proof must contain 64 hex characters"));
    }
    let mut output = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let pair = std::str::from_utf8(pair)
            .map_err(|_| key_proof("role-key proof contains invalid hex"))?;
        output[index] = u8::from_str_radix(pair, 16)
            .map_err(|_| key_proof("role-key proof contains invalid hex"))?;
    }
    Ok(output)
}

fn hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}

fn key_config(message: impl Into<String>) -> Error {
    Error::coded("KEY_CONFIGURATION_INVALID", message)
}

fn key_proof(message: impl Into<String>) -> Error {
    Error::coded("KEY_PROOF_INVALID", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> KeyRing {
        KeyRing::active_local_authority_profile([
            ("audit-v1".into(), vec![0x11; 32]),
            ("authorization-v1".into(), vec![0x22; 32]),
        ])
        .unwrap()
    }

    #[test]
    fn role_proofs_are_purpose_bound_and_tamper_evident() {
        let keys = profile();
        let proof = keys
            .sign(KeyPurpose::AuthorizationMac, b"authorized")
            .unwrap();
        keys.verify(
            &proof.key_id,
            KeyPurpose::AuthorizationMac,
            b"authorized",
            &proof.proof,
        )
        .unwrap();
        assert_eq!(
            keys.verify(
                &proof.key_id,
                KeyPurpose::AuditSealing,
                b"authorized",
                &proof.proof,
            )
            .unwrap_err()
            .code(),
            "KEY_ROLE_MISMATCH"
        );
        assert_eq!(
            keys.verify(
                &proof.key_id,
                KeyPurpose::AuthorizationMac,
                b"mutated",
                &proof.proof,
            )
            .unwrap_err()
            .code(),
            "KEY_PROOF_INVALID"
        );
    }

    #[test]
    fn configuration_rejects_reuse_missing_roles_and_multiple_active_keys() {
        let reused = KeyRing::active_local_authority_profile([
            ("audit-v1".into(), vec![0x11; 32]),
            ("authorization-v1".into(), vec![0x11; 32]),
        ])
        .unwrap_err();
        assert_eq!(reused.code(), "KEY_CONFIGURATION_INVALID");

        let missing = KeyRing::new(vec![RoleKey::active(
            "audit-v1",
            KeyPurpose::AuditSealing,
            vec![0x11; 32],
        )])
        .unwrap_err();
        assert_eq!(missing.code(), "KEY_CONFIGURATION_INVALID");

        let mut keys = profile().keys.into_values().collect::<Vec<_>>();
        keys.push(RoleKey::active(
            "audit-v2",
            KeyPurpose::AuditSealing,
            vec![0x66; 32],
        ));
        let duplicate = KeyRing::new(keys).unwrap_err();
        assert_eq!(duplicate.code(), "KEY_CONFIGURATION_INVALID");
    }

    #[test]
    fn verify_only_supports_rotation_while_revoked_keys_fail_closed() {
        let rotated = KeyRing::new(vec![
            RoleKey::verify_only("audit-v1", KeyPurpose::AuditSealing, vec![0x11; 32]),
            RoleKey::active("audit-v2", KeyPurpose::AuditSealing, vec![0x12; 32]),
            RoleKey::active(
                "authorization-v1",
                KeyPurpose::AuthorizationMac,
                vec![0x22; 32],
            ),
        ])
        .unwrap();
        assert_eq!(
            rotated
                .key_for_use("audit-v1", KeyPurpose::AuditSealing, true)
                .unwrap_err()
                .code(),
            "KEY_NOT_ACTIVE"
        );

        let revoked = KeyRing::new(vec![
            RoleKey::revoked("audit-v1", KeyPurpose::AuditSealing, vec![0x11; 32]),
            RoleKey::active("audit-v2", KeyPurpose::AuditSealing, vec![0x12; 32]),
            RoleKey::active(
                "authorization-v1",
                KeyPurpose::AuthorizationMac,
                vec![0x22; 32],
            ),
        ])
        .unwrap();
        assert_eq!(
            revoked
                .verify(
                    "audit-v1",
                    KeyPurpose::AuditSealing,
                    b"payload",
                    &format!("hmac-sha256:{}", "00".repeat(32)),
                )
                .unwrap_err()
                .code(),
            "KEY_REVOKED"
        );
    }
}
