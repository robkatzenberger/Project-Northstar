//! TL-PX 0.2 authority skeleton.
//!
//! This crate is not a 0.1 rewrite and not a PEP. It is a durable local
//! evaluate/authorize/claim authority with no side-effect capability.

pub mod authority;
pub mod error;
pub mod evidence;
pub mod hash;
pub mod jcs;
pub mod policy;
pub mod policy_manifest;
pub mod types;

pub use authority::{
    enforce_constraints, Authority, AuthorityConfig, AuthzState, ClaimRecord, EvaluationOutcome,
    IssuedAuthorization, Retryability,
};
pub use error::{Error, Result};
pub use evidence::{EvidenceConfig, EvidenceReconciliation, PartyType, SealedEvidence};
pub use hash::{
    approval_context_hash, assert_hash_string, authorized_action_hash, digest_hex,
    executed_action_hash, hash_canonical, hash_json_text, hash_value, intent_hash,
    policy_bundle_hash,
};
pub use jcs::{canonicalize, canonicalize_json_text, parse, utf8_hex, Canonical, Value};
pub use policy::{
    AuthorizationTemplate, CapabilityRegistry, Decision, PolicyBundle, PolicyEffect, PolicyRule,
    Principal, Switchboard,
};
pub use policy_manifest::{
    exact_match_policy_content_hash, ConfiguredPolicyBundle, PolicyBundleManifest, PolicyCatalog,
    PolicyIssuer, PolicyIssuerType, PolicySupersedes, SelectedPolicy,
    EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};
pub use types::{
    bindings_match, ActionBinding, Adapter, AuthorizedAction, ExecutedAction, Risk, SubmittedIntent,
};
