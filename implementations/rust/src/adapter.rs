//! Versioned integrity contracts for authenticated local adapters.

use crate::error::{Error, Result};
use crate::hash::assert_hash_string;
use crate::local_auth::{AuthenticatedIdentity, LocalAuthenticator, LocalRole};
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::net::UnixStream;

pub const ADAPTER_MATERIAL_FIELDS: [&str; 9] = [
    "executing_principal",
    "action",
    "target",
    "arguments",
    "environment",
    "tenant",
    "payload_hash",
    "artifact_hash",
    "adapter",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterContract {
    pub adapter_id: String,
    pub adapter_version: String,
    pub authenticated_principal: String,
    pub authenticated_authority: String,
    pub binary_hash: String,
    pub capabilities: Vec<String>,
    pub actions: Vec<String>,
    pub material_fields: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AdapterRegistry {
    contracts: BTreeMap<(String, String), AdapterContract>,
}

impl AdapterRegistry {
    pub fn new(contracts: Vec<AdapterContract>) -> Result<Self> {
        if contracts.is_empty() {
            return Err(adapter_error(
                "ADAPTER_MAPPING_INCOMPLETE",
                "adapter registry must not be empty",
            ));
        }
        let expected_fields = ADAPTER_MATERIAL_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect::<Vec<_>>();
        let mut registry = Self::default();
        for contract in contracts {
            for (name, value) in [
                ("adapter_id", contract.adapter_id.as_str()),
                ("adapter_version", contract.adapter_version.as_str()),
                (
                    "authenticated_principal",
                    contract.authenticated_principal.as_str(),
                ),
                (
                    "authenticated_authority",
                    contract.authenticated_authority.as_str(),
                ),
            ] {
                if value.is_empty() {
                    return Err(adapter_error(
                        "ADAPTER_MAPPING_INCOMPLETE",
                        format!("{name} must be non-empty"),
                    ));
                }
            }
            assert_hash_string(&contract.binary_hash).map_err(|_| {
                adapter_error(
                    "ADAPTER_INTEGRITY_INVALID",
                    "adapter binary hash must be canonical sha256",
                )
            })?;
            validate_unique_nonempty(&contract.capabilities, "capabilities")?;
            validate_unique_nonempty(&contract.actions, "actions")?;
            if contract.material_fields != expected_fields {
                return Err(adapter_error(
                    "ADAPTER_MAPPING_INCOMPLETE",
                    "adapter material fields are missing, reordered, or extended",
                ));
            }
            let key = (
                contract.adapter_id.clone(),
                contract.adapter_version.clone(),
            );
            if registry.contracts.insert(key, contract).is_some() {
                return Err(adapter_error(
                    "ADAPTER_MAPPING_INCOMPLETE",
                    "duplicate adapter id/version contract",
                ));
            }
        }
        Ok(registry)
    }

    pub(crate) fn covers_any(&self, adapter_id: &str, capability: &str, action: &str) -> bool {
        self.contracts.values().any(|contract| {
            contract.adapter_id == adapter_id
                && contract
                    .capabilities
                    .iter()
                    .any(|value| value == capability)
                && contract.actions.iter().any(|value| value == action)
        })
    }

    pub(crate) fn covers_capability(&self, adapter_id: &str, capability: &str) -> bool {
        self.contracts.values().any(|contract| {
            contract.adapter_id == adapter_id
                && contract
                    .capabilities
                    .iter()
                    .any(|value| value == capability)
        })
    }

    pub(crate) fn verify(
        &self,
        adapter_id: &str,
        adapter_version: &str,
        session: &AuthenticatedAdapterSession,
        binary_hash: &str,
        capability: &str,
        action: &str,
    ) -> Result<()> {
        let contract = self
            .contracts
            .get(&(adapter_id.to_string(), adapter_version.to_string()))
            .ok_or_else(|| {
                adapter_error(
                    "ADAPTER_VERSION_MISMATCH",
                    "adapter id/version is not activated",
                )
            })?;
        if contract.authenticated_principal != session.adapter_principal() {
            return Err(adapter_error(
                "ADAPTER_AUTHENTICATION_FAILED",
                "authenticated adapter principal does not match the contract",
            ));
        }
        if contract.authenticated_authority != session.authority_principal() {
            return Err(adapter_error(
                "ADAPTER_AUTHENTICATION_FAILED",
                "adapter did not authenticate the configured authority principal",
            ));
        }
        if contract.binary_hash != binary_hash {
            return Err(adapter_error(
                "ADAPTER_INTEGRITY_INVALID",
                "adapter binary hash does not match the activated contract",
            ));
        }
        if !contract
            .capabilities
            .iter()
            .any(|value| value == capability)
        {
            return Err(adapter_error(
                "ADAPTER_CAPABILITY_DENIED",
                "adapter contract does not cover the protected capability",
            ));
        }
        if !contract.actions.iter().any(|value| value == action) {
            return Err(adapter_error(
                "ADAPTER_MAPPING_INCOMPLETE",
                "adapter contract does not map this action",
            ));
        }
        Ok(())
    }
}

/// A local channel on which the authority authenticated the adapter peer and
/// the adapter independently authenticated the authority peer.
#[derive(Debug, Clone)]
pub struct AuthenticatedAdapterSession {
    adapter: AuthenticatedIdentity,
    authority: AuthenticatedIdentity,
}

impl AuthenticatedAdapterSession {
    pub fn authenticate_local(
        authority_authenticator: &LocalAuthenticator,
        adapter_authenticator: &LocalAuthenticator,
        authority_side: &UnixStream,
        adapter_side: &UnixStream,
    ) -> Result<Self> {
        let adapter = authority_authenticator.authenticate_stream(authority_side)?;
        adapter.require_role(LocalRole::Adapter)?;
        let authority = adapter_authenticator.authenticate_stream(adapter_side)?;
        authority.require_role(LocalRole::Authority)?;
        Ok(Self { adapter, authority })
    }

    pub fn adapter_principal(&self) -> &str {
        self.adapter.principal_id()
    }

    pub fn authority_principal(&self) -> &str {
        self.authority.principal_id()
    }
}

fn validate_unique_nonempty(values: &[String], name: &str) -> Result<()> {
    if values.is_empty() || values.iter().any(|value| value.is_empty()) {
        return Err(adapter_error(
            "ADAPTER_MAPPING_INCOMPLETE",
            format!("adapter {name} must be non-empty"),
        ));
    }
    let unique: BTreeSet<_> = values.iter().collect();
    if unique.len() != values.len() {
        return Err(adapter_error(
            "ADAPTER_MAPPING_INCOMPLETE",
            format!("adapter {name} must be unique"),
        ));
    }
    Ok(())
}

fn adapter_error(code: &str, message: impl Into<String>) -> Error {
    Error::coded(code, message)
}
