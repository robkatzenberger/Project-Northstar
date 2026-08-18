//! Kernel-derived local peer authentication for the initial Unix profile.
//!
//! Callers never choose their TL-PX principal in a request body. A configured
//! exact UID/GID mapping produces an opaque authenticated identity with bounded
//! roles and approval-route membership.

use crate::error::{Error, Result};
use crate::evidence::PartyType;
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::net::UnixStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LocalRole {
    Requester,
    Operator,
    Executor,
    EmergencyCanceller,
}

#[derive(Debug, Clone)]
pub struct LocalPrincipalMapping {
    pub uid: u32,
    pub gid: u32,
    pub principal_id: String,
    pub party_type: PartyType,
    pub roles: Vec<LocalRole>,
    pub approval_routes: Vec<String>,
}

#[derive(Debug, Clone)]
struct ValidatedMapping {
    principal_id: String,
    party_type: PartyType,
    roles: BTreeSet<LocalRole>,
    approval_routes: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct LocalAuthenticator {
    mappings: BTreeMap<(u32, u32), ValidatedMapping>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedIdentity {
    uid: u32,
    gid: u32,
    principal_id: String,
    party_type: PartyType,
    roles: BTreeSet<LocalRole>,
    approval_routes: BTreeSet<String>,
}

impl LocalAuthenticator {
    pub fn new(mappings: Vec<LocalPrincipalMapping>) -> Result<Self> {
        if mappings.is_empty() {
            return Err(Error::coded(
                "AUTHENTICATION_FAILED",
                "local identity mapping must not be empty",
            ));
        }
        let mut by_peer = BTreeMap::new();
        let mut principals = BTreeSet::new();
        for mapping in mappings {
            if mapping.principal_id.is_empty() {
                return Err(Error::coded(
                    "AUTHENTICATION_FAILED",
                    "mapped principal id must be non-empty",
                ));
            }
            if mapping.roles.is_empty() {
                return Err(Error::coded(
                    "AUTHENTICATION_FAILED",
                    "mapped principal must have at least one role",
                ));
            }
            if mapping.approval_routes.iter().any(|route| route.is_empty()) {
                return Err(Error::coded(
                    "AUTHENTICATION_FAILED",
                    "approval route identifiers must be non-empty",
                ));
            }
            let roles: BTreeSet<_> = mapping.roles.into_iter().collect();
            let routes: BTreeSet<_> = mapping.approval_routes.into_iter().collect();
            if !roles.contains(&LocalRole::Operator) && !routes.is_empty() {
                return Err(Error::coded(
                    "AUTHENTICATION_FAILED",
                    "only an operator mapping may carry approval routes",
                ));
            }
            if !principals.insert(mapping.principal_id.clone()) {
                return Err(Error::coded(
                    "AUTHENTICATION_FAILED",
                    "principal id must map to exactly one local peer identity",
                ));
            }
            let key = (mapping.uid, mapping.gid);
            if by_peer
                .insert(
                    key,
                    ValidatedMapping {
                        principal_id: mapping.principal_id,
                        party_type: mapping.party_type,
                        roles,
                        approval_routes: routes,
                    },
                )
                .is_some()
            {
                return Err(Error::coded(
                    "AUTHENTICATION_FAILED",
                    "duplicate UID/GID mapping",
                ));
            }
        }
        Ok(Self { mappings: by_peer })
    }

    pub fn authenticate_stream(&self, stream: &UnixStream) -> Result<AuthenticatedIdentity> {
        let (uid, gid) = peer_uid_gid(stream)?;
        let mapping = self.mappings.get(&(uid, gid)).ok_or_else(|| {
            Error::coded(
                "AUTHENTICATION_FAILED",
                "local peer UID/GID is not mapped to a TL-PX principal",
            )
        })?;
        Ok(AuthenticatedIdentity {
            uid,
            gid,
            principal_id: mapping.principal_id.clone(),
            party_type: mapping.party_type,
            roles: mapping.roles.clone(),
            approval_routes: mapping.approval_routes.clone(),
        })
    }
}

#[cfg(target_vendor = "apple")]
fn peer_uid_gid(stream: &UnixStream) -> Result<(u32, u32)> {
    let (uid, gid) = nix::unistd::getpeereid(stream).map_err(|_| {
        Error::coded(
            "AUTHENTICATION_FAILED",
            "operating-system peer credentials are unavailable",
        )
    })?;
    Ok((uid.as_raw(), gid.as_raw()))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn peer_uid_gid(stream: &UnixStream) -> Result<(u32, u32)> {
    let credentials =
        nix::sys::socket::getsockopt(stream, nix::sys::socket::sockopt::PeerCredentials).map_err(
            |_| {
                Error::coded(
                    "AUTHENTICATION_FAILED",
                    "operating-system peer credentials are unavailable",
                )
            },
        )?;
    Ok((credentials.uid(), credentials.gid()))
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux", target_os = "android")))]
fn peer_uid_gid(_stream: &UnixStream) -> Result<(u32, u32)> {
    Err(Error::coded(
        "AUTHENTICATION_FAILED",
        "this platform has no configured safe peer-credential implementation",
    ))
}

impl AuthenticatedIdentity {
    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

    pub fn uid(&self) -> u32 {
        self.uid
    }

    pub fn gid(&self) -> u32 {
        self.gid
    }

    pub fn party_type(&self) -> PartyType {
        self.party_type
    }

    pub fn has_role(&self, role: LocalRole) -> bool {
        self.roles.contains(&role)
    }

    pub fn permits_approval_route(&self, route: &str) -> bool {
        self.roles.contains(&LocalRole::Operator) && self.approval_routes.contains(route)
    }

    pub(crate) fn require_role(&self, role: LocalRole) -> Result<()> {
        if self.has_role(role) {
            Ok(())
        } else {
            Err(Error::coded(
                "AUTHENTICATION_FAILED",
                "authenticated local principal lacks the required role",
            ))
        }
    }
}
