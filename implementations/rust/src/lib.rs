//! TL-PX 0.2 authority skeleton.
//!
//! This crate is not a 0.1 rewrite. It contains the durable local authority and
//! a bounded cooperative shell-runner prototype; it is not forced mediation.

pub mod adapter;
pub mod authority;
pub mod error;
pub mod evidence;
pub mod hash;
pub mod jcs;
pub mod keys;
pub mod local_auth;
pub mod policy;
pub mod policy_manifest;
pub mod restricted_pep;
pub mod shell_runner;
pub mod types;

pub use adapter::{
    AdapterContract, AdapterRegistry, AuthenticatedAdapterSession, ADAPTER_MATERIAL_FIELDS,
};
pub use authority::{
    enforce_constraints, ApprovalOutcome, ApprovalPresentation, ApprovalResolution, ApprovalState,
    Authority, AuthorityConfig, AuthzState, CancellationOutcome, CancellationReason,
    CancellationRecord, CancellationRole, ClaimRecord, EvaluationOutcome, ExecutionLease,
    ExecutionReceipt, ExecutionResultEvidence, ExecutionStart, ExecutionState, IssuedAuthorization,
    PendingApprovalView, Retryability, RevocationReason, RevocationRecord, RevocationScope,
};
pub use error::{Error, Result};
pub use evidence::{EvidenceConfig, EvidenceReconciliation, PartyType, SealedEvidence};
pub use hash::{
    approval_context_hash, assert_hash_string, authorized_action_hash, digest_hex,
    executed_action_hash, hash_canonical, hash_json_text, hash_value, intent_hash,
    policy_bundle_hash,
};
pub use jcs::{canonicalize, canonicalize_json_text, parse, utf8_hex, Canonical, Value};
pub use keys::{KeyProof, KeyPurpose, KeyRing, KeyState, RoleKey};
pub use local_auth::{AuthenticatedIdentity, LocalAuthenticator, LocalPrincipalMapping, LocalRole};
pub use policy::{
    AuthorizationTemplate, CapabilityRegistry, Decision, PolicyBundle, PolicyEffect, PolicyRule,
    Principal, Switchboard,
};
pub use policy_manifest::{
    exact_match_policy_content_hash, ConfiguredPolicyBundle, PolicyBundleManifest, PolicyCatalog,
    PolicyIssuer, PolicyIssuerType, PolicySupersedes, SelectedPolicy,
    EXACT_MATCH_POLICY_CONTENT_TYPE, POLICY_PRECEDENCE,
};
pub use restricted_pep::{
    assert_replacement_bind_denied, raw_restricted_pep_request, request_restricted_pep,
    serve_restricted_pep, verify_restricted_pep_evidence, RestrictedPepConfig,
};
pub use shell_runner::{
    sha256_file, CooperativeShellConfig, CooperativeShellOutcome, CooperativeShellRequest,
    CooperativeShellRunner, ShellExecutable,
};
pub use types::{
    bindings_match, ActionBinding, Adapter, AuthorizedAction, ExecutedAction, Risk, SubmittedIntent,
};
