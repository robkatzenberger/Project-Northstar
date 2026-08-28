//! Durable local TL-PX authority MVP.
//!
//! A single SQLite transaction owns each evaluation outcome. Authenticated
//! idempotency slots are immutable, and one authority-wide sequence orders
//! decisions, evaluation errors, and successful claims. Public mutation paths
//! accept identities produced by an authenticator; construction and protection
//! of that authenticator remain deployment duties.

use crate::adapter::{AdapterRegistry, AuthenticatedAdapterSession};
use crate::error::{Error, Result};
use crate::evidence::{
    self, ApprovalEvidenceInput, CancellationEvidenceInput, DecisionEvidenceInput,
    ErrorEvidenceInput, EvidenceConfig, EvidenceReconciliation, SealedEvidence,
};
use crate::hash::assert_hash_string;
use crate::jcs::canonicalize;
use crate::keys::KeyPurpose;
use crate::local_auth::{AuthenticatedIdentity, LocalRole};
use crate::policy::{CapabilityRegistry, Decision, PolicyEffect, Switchboard};
use crate::policy_manifest::{ConfiguredPolicyBundle, PolicyCatalog};
use crate::types::{ActionBinding, AuthorizedAction, ExecutedAction, SubmittedIntent};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthzState {
    AuthorizedUnclaimed,
    Claimed,
    Expired,
    Revoked,
}

impl AuthzState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuthorizedUnclaimed => "AUTHORIZED_UNCLAIMED",
            Self::Claimed => "CLAIMED",
            Self::Expired => "EXPIRED",
            Self::Revoked => "REVOKED",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "AUTHORIZED_UNCLAIMED" => Ok(Self::AuthorizedUnclaimed),
            "CLAIMED" => Ok(Self::Claimed),
            "EXPIRED" => Ok(Self::Expired),
            "REVOKED" => Ok(Self::Revoked),
            _ => Err(Error::authority(format!(
                "unknown authorization state {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retryability {
    Never,
    AfterCondition,
    Immediate,
}

impl Retryability {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Never => "NEVER",
            Self::AfterCondition => "AFTER_CONDITION",
            Self::Immediate => "IMMEDIATE",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "NEVER" => Ok(Self::Never),
            "AFTER_CONDITION" => Ok(Self::AfterCondition),
            "IMMEDIATE" => Ok(Self::Immediate),
            _ => Err(Error::authority(format!("unknown retryability {value}"))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthorityConfig {
    pub policy: PolicyCatalog,
    pub switchboard: Switchboard,
    pub capabilities: CapabilityRegistry,
    pub adapters: AdapterRegistry,
    pub evidence: EvidenceConfig,
    pub approval_window_ms: i64,
    pub claim_window_ms: i64,
    pub execution_lease_ms: i64,
}

impl AuthorityConfig {
    fn validate(&self) -> Result<()> {
        self.policy.validate()?;
        self.evidence.validate()?;
        for template in self.policy.authorization_templates() {
            if !self.capabilities.contains(&template.capability) {
                return Err(Error::policy_compile(format!(
                    "ALLOW capability {} is absent from capability registry",
                    template.capability
                )));
            }
        }
        for (capability, adapter_ids) in self.capabilities.entries() {
            for adapter_id in adapter_ids {
                if !self.adapters.covers_capability(adapter_id, capability) {
                    return Err(Error::policy_compile(format!(
                        "capability {capability} names adapter {adapter_id} without an integrity contract"
                    )));
                }
            }
        }
        for configured in &self.policy.bundles {
            if configured.policy.default.authorization.is_some() {
                return Err(Error::policy_compile(
                    "adapter-integrity profile requires explicit action rules for authorization",
                ));
            }
            for rule in &configured.policy.rules {
                let Some(template) = rule.effect.authorization.as_ref() else {
                    continue;
                };
                let covered = self
                    .capabilities
                    .entries()
                    .find(|(capability, _)| *capability == template.capability)
                    .is_some_and(|(_, adapter_ids)| {
                        adapter_ids.iter().any(|adapter_id| {
                            self.adapters
                                .covers_any(adapter_id, &template.capability, &rule.action)
                        })
                    });
                if !covered {
                    return Err(Error::policy_compile(format!(
                        "action {} has no complete adapter integrity contract for capability {}",
                        rule.action, template.capability
                    )));
                }
            }
        }
        if self.approval_window_ms <= 0 || self.claim_window_ms <= 0 || self.execution_lease_ms <= 0
        {
            return Err(Error::policy_compile(
                "approval, claim, and execution windows must be positive",
            ));
        }
        if self.execution_lease_ms % 1_000 != 0 {
            return Err(Error::policy_compile(
                "execution lease must be a whole number of seconds",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct IssuedAuthorization {
    pub authorization_id: String,
    pub receipt_id: String,
    pub requesting_principal: String,
    pub executing_principal: String,
    pub request_id: String,
    pub action: String,
    pub target: String,
    pub intent_hash: String,
    pub authorized_action_hash: String,
    pub action_binding_hash: String,
    pub capability: String,
    pub policy_bundle_hash: String,
    pub resource_scope: Vec<String>,
    pub adapter_id: String,
    pub adapter_version: String,
    pub environment: String,
    pub tenant: String,
    pub authorization_nonce: String,
    /// Authority-local proof metadata. These fields are not private additions
    /// to the portable `tlpx.authorization` JSON record.
    pub authorization_signing_key_id: String,
    pub authorization_signature: String,
    pub idempotency_key: String,
    pub issued_at_ms: i64,
    pub claim_expires_at_ms: i64,
    pub execution_lease_ms: i64,
    pub state: AuthzState,
}

#[derive(Debug, Clone)]
pub struct EvaluationOutcome {
    pub receipt_id: String,
    pub sequence: i64,
    pub authenticated_requester: String,
    pub request_id: String,
    pub retry_of_receipt_id: Option<String>,
    pub decision: Decision,
    pub reason_code: String,
    pub policy_id: Option<String>,
    pub intent_hash: String,
    pub policy_bundle_hash: String,
    pub authorization: Option<IssuedAuthorization>,
}

#[derive(Debug, Clone)]
pub struct ClaimRecord {
    pub claim_id: String,
    pub authorization_id: String,
    pub receipt_id: String,
    pub sequence: i64,
    pub executing_principal: String,
    pub authorized_action_hash: String,
    pub action_binding_hash: String,
    pub executed_action_hash: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub claimed_at_ms: i64,
    pub lease_expires_at_ms: i64,
}

/// A durable authority-local revocation scope. This is deliberately not a
/// claim that the deferred portable `tlpx.revocation` record exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationScope {
    Authorization,
    Principal,
    PolicyBundle,
    SigningKey,
    Tenant,
    Environment,
    Capability,
}

impl RevocationScope {
    fn as_str(self) -> &'static str {
        match self {
            Self::Authorization => "AUTHORIZATION",
            Self::Principal => "PRINCIPAL",
            Self::PolicyBundle => "POLICY_BUNDLE",
            Self::SigningKey => "SIGNING_KEY",
            Self::Tenant => "TENANT",
            Self::Environment => "ENVIRONMENT",
            Self::Capability => "CAPABILITY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationReason {
    AuthorizationWithdrawn,
    PrincipalDisabled,
    PolicyRetired,
    SigningKeyCompromised,
    TenantDisabled,
    EnvironmentDisabled,
    CapabilityDisabled,
    EmergencyDeny,
}

impl RevocationReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::AuthorizationWithdrawn => "AUTHORIZATION_WITHDRAWN",
            Self::PrincipalDisabled => "PRINCIPAL_DISABLED",
            Self::PolicyRetired => "POLICY_RETIRED",
            Self::SigningKeyCompromised => "SIGNING_KEY_COMPROMISED",
            Self::TenantDisabled => "TENANT_DISABLED",
            Self::EnvironmentDisabled => "ENVIRONMENT_DISABLED",
            Self::CapabilityDisabled => "CAPABILITY_DISABLED",
            Self::EmergencyDeny => "EMERGENCY_DENY",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationRecord {
    pub revocation_id: String,
    pub sequence: i64,
    pub scope: RevocationScope,
    pub scope_id: String,
    pub revoking_principal: String,
    pub reason: RevocationReason,
    pub revoked_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    Started,
    ExecutionOutcomeUnknown,
    ReconciliationRequired,
    Completed,
    Failed,
    Cancelled,
    LeaseExpired,
    CompletedConfirmed,
    FailedConfirmed,
    OutcomeUnknownFinal,
}

impl ExecutionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Started => "STARTED",
            Self::ExecutionOutcomeUnknown => "EXECUTION_OUTCOME_UNKNOWN",
            Self::ReconciliationRequired => "RECONCILIATION_REQUIRED",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
            Self::LeaseExpired => "LEASE_EXPIRED",
            Self::CompletedConfirmed => "COMPLETED_CONFIRMED",
            Self::FailedConfirmed => "FAILED_CONFIRMED",
            Self::OutcomeUnknownFinal => "OUTCOME_UNKNOWN_FINAL",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "STARTED" => Ok(Self::Started),
            "EXECUTION_OUTCOME_UNKNOWN" => Ok(Self::ExecutionOutcomeUnknown),
            "RECONCILIATION_REQUIRED" => Ok(Self::ReconciliationRequired),
            "COMPLETED" => Ok(Self::Completed),
            "FAILED" => Ok(Self::Failed),
            "CANCELLED" => Ok(Self::Cancelled),
            "LEASE_EXPIRED" => Ok(Self::LeaseExpired),
            "COMPLETED_CONFIRMED" => Ok(Self::CompletedConfirmed),
            "FAILED_CONFIRMED" => Ok(Self::FailedConfirmed),
            "OUTCOME_UNKNOWN_FINAL" => Ok(Self::OutcomeUnknownFinal),
            _ => Err(Error::authority(format!("unknown execution state {value}"))),
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::Failed
                | Self::Cancelled
                | Self::LeaseExpired
                | Self::CompletedConfirmed
                | Self::FailedConfirmed
                | Self::OutcomeUnknownFinal
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationOutcome {
    CancelledBeforeSideEffect,
    CancellationRequested,
    CancelledDuringExecution,
    CancellationUnsupported,
    CompletedBeforeCancellation,
}

impl CancellationOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CancelledBeforeSideEffect => "CANCELLED_BEFORE_SIDE_EFFECT",
            Self::CancellationRequested => "CANCELLATION_REQUESTED",
            Self::CancelledDuringExecution => "CANCELLED_DURING_EXECUTION",
            Self::CancellationUnsupported => "CANCELLATION_UNSUPPORTED",
            Self::CompletedBeforeCancellation => "COMPLETED_BEFORE_CANCELLATION",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "CANCELLED_BEFORE_SIDE_EFFECT" => Ok(Self::CancelledBeforeSideEffect),
            "CANCELLATION_REQUESTED" => Ok(Self::CancellationRequested),
            "CANCELLED_DURING_EXECUTION" => Ok(Self::CancelledDuringExecution),
            "CANCELLATION_UNSUPPORTED" => Ok(Self::CancellationUnsupported),
            "COMPLETED_BEFORE_CANCELLATION" => Ok(Self::CompletedBeforeCancellation),
            _ => Err(Error::authority(format!(
                "unknown cancellation outcome {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResultEvidence {
    pub result_summary: Option<String>,
    pub result_hash: Option<String>,
    pub external_evidence_reference: Option<String>,
}

impl ExecutionResultEvidence {
    fn validate(&self) -> Result<()> {
        if self
            .result_summary
            .as_ref()
            .is_some_and(|value| value.is_empty() || value.chars().count() > 2_048)
            || self
                .external_evidence_reference
                .as_ref()
                .is_some_and(|value| value.is_empty() || value.chars().count() > 2_048)
        {
            return Err(Error::coded(
                "EXECUTION_RESULT_INVALID",
                "result summary and external evidence reference must be non-empty and bounded",
            ));
        }
        if let Some(result_hash) = &self.result_hash {
            assert_hash_string(result_hash).map_err(|_| {
                Error::coded(
                    "EXECUTION_RESULT_INVALID",
                    "result hash must be a canonical sha256 digest",
                )
            })?;
        }
        if self.result_summary.is_none()
            && self.result_hash.is_none()
            && self.external_evidence_reference.is_none()
        {
            return Err(Error::coded(
                "EXECUTION_RESULT_INVALID",
                "terminal execution requires bounded result evidence",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionLease {
    pub execution_id: String,
    pub claim_id: String,
    pub authorization_id: String,
    pub idempotency_key: String,
    pub executing_principal: String,
    pub started_at_ms: i64,
    pub lease_expires_at_ms: i64,
    pub state: ExecutionState,
}

/// Outcome of an execution-start request. Only `Started` authorizes this
/// caller to begin the protected side effect. `NotStarted` reports durable
/// state for a retry or a lease that closed before start; it is never spawn
/// permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStart {
    Started(ExecutionLease),
    NotStarted(ExecutionLease),
}

impl ExecutionStart {
    /// Returns the lease only when this call created the durable `STARTED`
    /// transition. Retried or already-closed attempts fail closed.
    pub fn into_started(self) -> Result<ExecutionLease> {
        match self {
            Self::Started(lease) => Ok(lease),
            Self::NotStarted(_) => Err(Error::coded(
                "EXECUTION_STATE_INVALID",
                "this call did not create the durable STARTED transition",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReceipt {
    pub execution_id: String,
    pub claim_id: String,
    pub authorization_id: String,
    pub receipt_id: String,
    pub requesting_principal: String,
    pub executing_principal: String,
    pub intent_hash: String,
    pub authorized_action_hash: String,
    pub executed_action_hash: String,
    pub target: String,
    pub policy_bundle_id: String,
    pub policy_bundle_version: String,
    pub policy_bundle_hash: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub adapter_principal: Option<String>,
    pub adapter_binary_hash: Option<String>,
    pub sequence: i64,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub state: ExecutionState,
    pub result: ExecutionResultEvidence,
    pub cancellation_outcome: Option<CancellationOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationRole {
    Requester,
    Operator,
    EmergencyAuthority,
}

impl CancellationRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::Requester => "REQUESTER",
            Self::Operator => "OPERATOR",
            Self::EmergencyAuthority => "EMERGENCY_AUTHORITY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationReason {
    RequesterWithdrawn,
    OperatorCancelled,
    EmergencyRevocation,
}

impl CancellationReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::RequesterWithdrawn => "REQUESTER_WITHDRAWN",
            Self::OperatorCancelled => "OPERATOR_CANCELLED",
            Self::EmergencyRevocation => "EMERGENCY_REVOCATION",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationRecord {
    pub receipt_id: String,
    pub sequence: i64,
    pub canceller: String,
    pub canceller_role: CancellationRole,
    pub reason: CancellationReason,
    pub cancelled_at_ms: i64,
    pub policy_bundle_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalOutcome {
    Approve,
    Reject,
}

impl ApprovalOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "APPROVE",
            Self::Reject => "REJECT",
        }
    }

    fn terminal_state(self) -> ApprovalState {
        match self {
            Self::Approve => ApprovalState::Approved,
            Self::Reject => ApprovalState::Rejected,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    Pending,
    Approved,
    Rejected,
    Cancelled,
    Expired,
}

impl ApprovalState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING_APPROVAL",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
            Self::Cancelled => "CANCELLED",
            Self::Expired => "APPROVAL_EXPIRED",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "PENDING_APPROVAL" => Ok(Self::Pending),
            "APPROVED" => Ok(Self::Approved),
            "REJECTED" => Ok(Self::Rejected),
            "CANCELLED" => Ok(Self::Cancelled),
            "APPROVAL_EXPIRED" => Ok(Self::Expired),
            _ => Err(Error::authority(format!("unknown approval state {value}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalPresentation {
    pub authorized_action_hash: String,
    pub renderer_id: String,
    pub renderer_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingApprovalView {
    pub receipt_id: String,
    pub requesting_principal: String,
    pub executing_principal: String,
    pub action: String,
    pub target: String,
    pub authorized_action_json: String,
    pub authorized_action_hash: String,
    pub action_binding_hash: String,
    pub approval_route: Vec<String>,
    pub approval_expires_at_ms: i64,
}

#[derive(Debug, Clone)]
pub struct ApprovalResolution {
    pub receipt_id: String,
    pub sequence: i64,
    pub outcome: ApprovalOutcome,
    pub operator: String,
    pub acted_at_ms: i64,
    pub authorized_action_hash: String,
    pub policy_bundle_hash: String,
    pub approval_route: Vec<String>,
    pub renderer_id: String,
    pub renderer_version: String,
    pub authorization: Option<IssuedAuthorization>,
}

pub struct Authority {
    db: Mutex<Connection>,
    config: AuthorityConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalSnapshot {
    pub trusted_time_last_ms: Option<i64>,
    pub evidence: EvidenceReconciliation,
    pub pending_approvals: i64,
    pub unclaimed_authorizations: i64,
    pub started_executions: i64,
    pub outcome_unknown_executions: i64,
    pub reconciliation_required_executions: i64,
    pub active_revocations: i64,
}

impl Authority {
    pub fn open(path: impl AsRef<Path>, config: AuthorityConfig) -> Result<Self> {
        config.validate()?;
        let connection = Connection::open(path).map_err(db_error)?;
        Self::from_connection(connection, config)
    }

    pub fn in_memory(config: AuthorityConfig) -> Result<Self> {
        config.validate()?;
        let connection = Connection::open_in_memory().map_err(db_error)?;
        Self::from_connection(connection, config)
    }

    fn from_connection(connection: Connection, config: AuthorityConfig) -> Result<Self> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(db_error)?;
        verify_existing_schema_compatibility(&connection)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 CREATE TABLE IF NOT EXISTS tlpx_sequence (
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   next_value INTEGER NOT NULL CHECK(next_value >= 1)
                 );
                 INSERT OR IGNORE INTO tlpx_sequence (singleton, next_value) VALUES (1, 1);
                 CREATE TABLE IF NOT EXISTS tlpx_trusted_time (
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   last_observed_ms INTEGER NOT NULL CHECK(last_observed_ms >= 0)
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_evaluations (
                   sequence INTEGER PRIMARY KEY,
                   receipt_id TEXT NOT NULL UNIQUE,
                   authenticated_principal TEXT,
                   request_id TEXT,
                   intent_hash TEXT,
                   retry_of_receipt_id TEXT REFERENCES tlpx_evaluations(receipt_id),
                   outcome_kind TEXT NOT NULL CHECK(outcome_kind IN ('DECISION','EVALUATION_ERROR')),
                   decision TEXT CHECK(decision IN ('ALLOW','REQUIRE_APPROVAL','DENY')),
                   policy_id TEXT,
                   stage TEXT,
                   reason_code TEXT NOT NULL,
                   reason TEXT NOT NULL,
                   retryability TEXT CHECK(retryability IN ('NEVER','AFTER_CONDITION','IMMEDIATE')),
                   required_condition TEXT,
                   policy_bundle_hash TEXT,
                   evaluated_at_ms INTEGER NOT NULL,
                   authorization_id TEXT,
                   occupies_slot INTEGER NOT NULL CHECK(occupies_slot IN (0, 1)),
                   CHECK((authenticated_principal IS NULL) = (request_id IS NULL)),
                   CHECK(occupies_slot = 0 OR authenticated_principal IS NOT NULL),
                   CHECK(
                     (outcome_kind = 'DECISION' AND decision IS NOT NULL AND stage IS NULL
                       AND retryability IS NULL
                       AND required_condition IS NULL AND intent_hash IS NOT NULL
                       AND policy_bundle_hash IS NOT NULL)
                     OR
                     (outcome_kind = 'EVALUATION_ERROR' AND decision IS NULL AND stage IS NOT NULL
                       AND retryability IS NOT NULL
                       AND ((retryability = 'AFTER_CONDITION' AND required_condition IS NOT NULL)
                         OR (retryability != 'AFTER_CONDITION' AND required_condition IS NULL)))
                   )
                 );
                 CREATE UNIQUE INDEX IF NOT EXISTS tlpx_evaluation_idempotency
                   ON tlpx_evaluations(authenticated_principal, request_id)
                   WHERE occupies_slot = 1;
                 CREATE TABLE IF NOT EXISTS tlpx_pending_approvals (
                   receipt_id TEXT PRIMARY KEY REFERENCES tlpx_evaluations(receipt_id),
                   requesting_principal TEXT NOT NULL,
                   request_id TEXT NOT NULL,
                   intent_hash TEXT NOT NULL,
                   policy_bundle_hash TEXT NOT NULL,
                   state TEXT NOT NULL CHECK(state IN (
                     'PENDING_APPROVAL','APPROVED','REJECTED','CANCELLED','APPROVAL_EXPIRED'
                   )),
                   approval_expires_at_ms INTEGER NOT NULL,
                   terminal_sequence INTEGER UNIQUE,
                   terminal_at_ms INTEGER,
                   terminal_actor TEXT,
                   terminal_actor_type TEXT CHECK(terminal_actor_type IN ('human','machine')),
                   terminal_role TEXT CHECK(terminal_role IN (
                     'REQUESTER','OPERATOR','EMERGENCY_AUTHORITY'
                   )),
                   terminal_reason TEXT,
                   executing_principal TEXT NOT NULL,
                   action TEXT NOT NULL,
                   target TEXT NOT NULL,
                   authorized_action_json TEXT NOT NULL,
                   authorized_action_hash TEXT NOT NULL,
                   action_binding_hash TEXT NOT NULL,
                   capability TEXT NOT NULL,
                   adapter_id TEXT NOT NULL,
                   adapter_version TEXT NOT NULL,
                   environment TEXT NOT NULL,
                   tenant TEXT NOT NULL,
                   execution_lease_ms INTEGER NOT NULL,
                   CHECK(
                     (state = 'PENDING_APPROVAL' AND terminal_sequence IS NULL
                       AND terminal_at_ms IS NULL AND terminal_actor IS NULL
                       AND terminal_actor_type IS NULL AND terminal_role IS NULL
                       AND terminal_reason IS NULL)
                     OR
                     (state = 'CANCELLED' AND terminal_sequence IS NOT NULL
                       AND terminal_at_ms IS NOT NULL AND terminal_actor IS NOT NULL
                       AND terminal_actor_type IS NOT NULL AND terminal_role IS NOT NULL
                       AND terminal_reason IS NOT NULL)
                     OR
                     (state IN ('APPROVED','REJECTED') AND terminal_sequence IS NOT NULL
                       AND terminal_at_ms IS NOT NULL AND terminal_actor IS NOT NULL
                       AND terminal_actor_type = 'human' AND terminal_role = 'OPERATOR'
                       AND terminal_reason IS NULL)
                     OR
                     (state = 'APPROVAL_EXPIRED' AND terminal_sequence IS NOT NULL
                       AND terminal_at_ms IS NOT NULL AND terminal_actor IS NULL
                       AND terminal_actor_type IS NULL AND terminal_role IS NULL
                       AND terminal_reason = 'APPROVAL_WINDOW_EXPIRED')
                   )
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_pending_approval_routes (
                   receipt_id TEXT NOT NULL REFERENCES tlpx_pending_approvals(receipt_id) ON DELETE CASCADE,
                   route_id TEXT NOT NULL,
                   position INTEGER NOT NULL CHECK(position >= 0),
                   PRIMARY KEY(receipt_id, route_id),
                   UNIQUE(receipt_id, position)
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_pending_approval_scopes (
                   receipt_id TEXT NOT NULL REFERENCES tlpx_pending_approvals(receipt_id) ON DELETE CASCADE,
                   resource TEXT NOT NULL,
                   position INTEGER NOT NULL CHECK(position >= 0),
                   PRIMARY KEY(receipt_id, resource),
                   UNIQUE(receipt_id, position)
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_operator_actions (
                   receipt_id TEXT PRIMARY KEY REFERENCES tlpx_pending_approvals(receipt_id),
                   sequence INTEGER NOT NULL UNIQUE,
                   outcome TEXT NOT NULL CHECK(outcome IN ('APPROVE','REJECT','CANCEL')),
                   actor_id TEXT NOT NULL,
                   actor_type TEXT NOT NULL CHECK(actor_type IN ('human','machine')),
                   actor_role TEXT NOT NULL CHECK(actor_role IN (
                     'REQUESTER','OPERATOR','EMERGENCY_AUTHORITY'
                   )),
                   reason TEXT,
                   policy_bundle_hash TEXT NOT NULL,
                   acted_at_ms INTEGER NOT NULL,
                   authorized_action_hash TEXT,
                   renderer_id TEXT,
                   renderer_version TEXT,
                   CHECK(
                     (outcome = 'CANCEL' AND reason IS NOT NULL
                       AND authorized_action_hash IS NULL
                       AND renderer_id IS NULL AND renderer_version IS NULL)
                     OR
                     (outcome IN ('APPROVE','REJECT') AND actor_type = 'human'
                       AND actor_role = 'OPERATOR' AND reason IS NULL
                       AND authorized_action_hash IS NOT NULL
                       AND renderer_id IS NOT NULL AND renderer_version IS NOT NULL)
                   )
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_authorizations (
                   authorization_id TEXT PRIMARY KEY,
                   receipt_id TEXT NOT NULL UNIQUE REFERENCES tlpx_evaluations(receipt_id),
                   requesting_principal TEXT NOT NULL,
                   executing_principal TEXT NOT NULL,
                   request_id TEXT NOT NULL,
                   action TEXT NOT NULL,
                   target TEXT NOT NULL,
                   intent_hash TEXT NOT NULL,
                   authorized_action_hash TEXT NOT NULL,
                   action_binding_hash TEXT NOT NULL,
                   capability TEXT NOT NULL,
                   policy_bundle_hash TEXT NOT NULL,
                   adapter_id TEXT NOT NULL,
                   adapter_version TEXT NOT NULL,
                   environment TEXT NOT NULL,
                   tenant TEXT NOT NULL,
                   authorization_nonce TEXT NOT NULL UNIQUE,
                   authorization_signing_key_id TEXT NOT NULL,
                   authorization_signature TEXT NOT NULL,
                   issued_at_ms INTEGER NOT NULL,
                   claim_expires_at_ms INTEGER NOT NULL,
                   execution_lease_ms INTEGER NOT NULL,
                   state TEXT NOT NULL CHECK(state IN ('AUTHORIZED_UNCLAIMED','CLAIMED','EXPIRED','REVOKED')),
                   revoked_at_ms INTEGER,
                   claimed_at_ms INTEGER,
                   claim_id TEXT UNIQUE,
                   presented_binding_hash TEXT
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_authorization_scopes (
                   authorization_id TEXT NOT NULL REFERENCES tlpx_authorizations(authorization_id) ON DELETE CASCADE,
                   resource TEXT NOT NULL,
                   position INTEGER NOT NULL CHECK(position >= 0),
                   PRIMARY KEY(authorization_id, resource),
                   UNIQUE(authorization_id, position)
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_claims (
                   claim_id TEXT PRIMARY KEY,
                   authorization_id TEXT NOT NULL UNIQUE REFERENCES tlpx_authorizations(authorization_id),
                   receipt_id TEXT NOT NULL REFERENCES tlpx_evaluations(receipt_id),
                   sequence INTEGER NOT NULL UNIQUE,
                   executing_principal TEXT NOT NULL,
                   authorized_action_hash TEXT NOT NULL,
                   action_binding_hash TEXT NOT NULL,
                   executed_action_hash TEXT NOT NULL,
                   adapter_id TEXT NOT NULL,
                   adapter_version TEXT NOT NULL,
                   claimed_at_ms INTEGER NOT NULL,
                   lease_expires_at_ms INTEGER NOT NULL,
                   CHECK(action_binding_hash = executed_action_hash)
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_revocations (
                   revocation_id TEXT PRIMARY KEY,
                   sequence INTEGER NOT NULL UNIQUE,
                   scope_type TEXT NOT NULL CHECK(scope_type IN (
                     'AUTHORIZATION','PRINCIPAL','POLICY_BUNDLE','SIGNING_KEY',
                     'TENANT','ENVIRONMENT','CAPABILITY'
                   )),
                   scope_id TEXT NOT NULL CHECK(length(scope_id) > 0),
                   revoking_principal TEXT NOT NULL CHECK(length(revoking_principal) > 0),
                   reason TEXT NOT NULL CHECK(reason IN (
                     'AUTHORIZATION_WITHDRAWN','PRINCIPAL_DISABLED','POLICY_RETIRED',
                     'SIGNING_KEY_COMPROMISED','TENANT_DISABLED','ENVIRONMENT_DISABLED',
                     'CAPABILITY_DISABLED','EMERGENCY_DENY'
                   )),
                   revoked_at_ms INTEGER NOT NULL CHECK(revoked_at_ms >= 0),
                   UNIQUE(scope_type, scope_id)
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_executions (
                   execution_id TEXT PRIMARY KEY,
                   claim_id TEXT NOT NULL UNIQUE REFERENCES tlpx_claims(claim_id),
                   authorization_id TEXT NOT NULL REFERENCES tlpx_authorizations(authorization_id),
                   receipt_id TEXT NOT NULL REFERENCES tlpx_evaluations(receipt_id),
                   idempotency_key TEXT NOT NULL UNIQUE,
                   requesting_principal TEXT NOT NULL,
                   executing_principal TEXT NOT NULL,
                   intent_hash TEXT NOT NULL,
                   authorized_action_hash TEXT NOT NULL,
                   executed_action_hash TEXT NOT NULL,
                   target TEXT NOT NULL,
                   policy_bundle_id TEXT NOT NULL,
                   policy_bundle_version TEXT NOT NULL,
                   policy_bundle_hash TEXT NOT NULL,
                   adapter_id TEXT NOT NULL,
                   adapter_version TEXT NOT NULL,
                   adapter_principal TEXT,
                   adapter_binary_hash TEXT,
                   started_at_ms INTEGER NOT NULL,
                   lease_expires_at_ms INTEGER NOT NULL,
                   state TEXT NOT NULL CHECK(state IN (
                     'STARTED','EXECUTION_OUTCOME_UNKNOWN','RECONCILIATION_REQUIRED',
                     'COMPLETED','FAILED','CANCELLED','LEASE_EXPIRED',
                     'COMPLETED_CONFIRMED','FAILED_CONFIRMED','OUTCOME_UNKNOWN_FINAL'
                   )),
                   outcome_unknown_at_ms INTEGER,
                   reconciliation_required_at_ms INTEGER,
                   terminal_sequence INTEGER UNIQUE,
                   ended_at_ms INTEGER,
                   result_summary TEXT,
                   result_hash TEXT,
                   external_evidence_reference TEXT,
                   cancellation_outcome TEXT CHECK(cancellation_outcome IS NULL OR cancellation_outcome IN (
                     'CANCELLED_BEFORE_SIDE_EFFECT','CANCELLATION_REQUESTED',
                     'CANCELLED_DURING_EXECUTION','CANCELLATION_UNSUPPORTED',
                     'COMPLETED_BEFORE_CANCELLATION'
                   )),
                   CHECK(lease_expires_at_ms > started_at_ms),
                   CHECK(
                     (state IN ('STARTED','EXECUTION_OUTCOME_UNKNOWN','RECONCILIATION_REQUIRED')
                       AND terminal_sequence IS NULL AND ended_at_ms IS NULL
                       AND result_summary IS NULL AND result_hash IS NULL
                       AND external_evidence_reference IS NULL AND cancellation_outcome IS NULL)
                     OR
                     (state IN ('COMPLETED','FAILED','CANCELLED','LEASE_EXPIRED',
                                'COMPLETED_CONFIRMED','FAILED_CONFIRMED','OUTCOME_UNKNOWN_FINAL')
                       AND terminal_sequence IS NOT NULL AND ended_at_ms IS NOT NULL
                       AND (result_summary IS NOT NULL OR result_hash IS NOT NULL
                            OR external_evidence_reference IS NOT NULL))
                   )
                 );
                 CREATE TABLE IF NOT EXISTS tlpx_evidence_outbox (
                   outbox_id INTEGER PRIMARY KEY AUTOINCREMENT,
                   authority_sequence INTEGER NOT NULL CHECK(authority_sequence >= 1),
                   ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
                   record_type TEXT NOT NULL CHECK(record_type IN (
                     'tlpx.decision', 'tlpx.evaluation_error',
                     'tlpx.authorization', 'tlpx.authorization_claim',
                     'tlpx.operator_action', 'tlpx.execution'
                   )),
                   source_id TEXT NOT NULL,
                   record_json TEXT NOT NULL,
                   record_hash TEXT NOT NULL,
                   previous_chain_hash TEXT,
                   chain_hash TEXT NOT NULL UNIQUE,
                   seal_algorithm TEXT NOT NULL CHECK(seal_algorithm = 'HMAC-SHA256'),
                   seal_key_id TEXT NOT NULL,
                   seal TEXT NOT NULL,
                   exported_at_ms INTEGER,
                   UNIQUE(authority_sequence, ordinal),
                   UNIQUE(record_type, source_id)
                 );",
            )
            .map_err(db_error)?;
        verify_existing_schema_compatibility(&connection)?;
        Ok(Self {
            db: Mutex::new(connection),
            config,
        })
    }

    fn evaluate_trusted_embedding(
        &self,
        authenticated_requester: &str,
        intent: &SubmittedIntent,
    ) -> Result<EvaluationOutcome> {
        self.evaluate_trusted_embedding_at(authenticated_requester, intent, now_ms()?)
    }

    pub fn evaluate_authenticated(
        &self,
        requester: &AuthenticatedIdentity,
        intent: &SubmittedIntent,
    ) -> Result<EvaluationOutcome> {
        requester.require_role(LocalRole::Requester)?;
        self.evaluate_trusted_embedding(requester.principal_id(), intent)
    }

    pub fn evaluate_authenticated_at(
        &self,
        requester: &AuthenticatedIdentity,
        intent: &SubmittedIntent,
        evaluated_at_ms: i64,
    ) -> Result<EvaluationOutcome> {
        requester.require_role(LocalRole::Requester)?;
        self.evaluate_trusted_embedding_at(requester.principal_id(), intent, evaluated_at_ms)
    }

    fn evaluate_trusted_embedding_at(
        &self,
        authenticated_requester: &str,
        intent: &SubmittedIntent,
        evaluated_at_ms: i64,
    ) -> Result<EvaluationOutcome> {
        let scoped_principal = nonempty(authenticated_requester);
        let scoped_request_id = nonempty(&intent.request_id);
        let current_intent_hash = intent.intent_hash().ok();
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;

        if let (Some(principal), Some(request_id)) = (scoped_principal, scoped_request_id) {
            if let Some(existing) = load_idempotent(&transaction, principal, request_id)? {
                let conflicts = match (&existing.intent_hash, &current_intent_hash) {
                    (Some(stored), Some(current)) => stored != current,
                    (Some(_), None) => true,
                    (None, _) => false,
                };
                if conflicts {
                    observe_trusted_time(&transaction, evaluated_at_ms)?;
                    return commit_evaluation_error(
                        transaction,
                        ErrorInput {
                            authenticated_principal: Some(principal),
                            request_id: Some(request_id),
                            intent_hash: current_intent_hash.as_deref(),
                            retry_of_receipt_id: None,
                            stage: "idempotency",
                            code: "IDEMPOTENCY_CONFLICT",
                            message: "request_id is already bound to a different intent",
                            retryability: Retryability::Never,
                            required_condition: None,
                            policy_bundle_id: None,
                            policy_bundle_hash: None,
                            evaluated_at_ms,
                            occupies_slot: false,
                        },
                        &self.config,
                    );
                }
                let result = existing.into_result(&transaction);
                transaction.commit().map_err(db_error)?;
                return result;
            }
        }
        observe_trusted_time(&transaction, evaluated_at_ms)?;

        if scoped_principal.is_none() {
            return commit_evaluation_error(
                transaction,
                ErrorInput {
                    authenticated_principal: None,
                    request_id: None,
                    intent_hash: None,
                    retry_of_receipt_id: None,
                    stage: "authentication",
                    code: "AUTHENTICATION_FAILED",
                    message: "authenticated requester context is unavailable",
                    retryability: Retryability::AfterCondition,
                    required_condition: Some("authenticated requester context is available"),
                    policy_bundle_id: None,
                    policy_bundle_hash: None,
                    evaluated_at_ms,
                    occupies_slot: false,
                },
                &self.config,
            );
        }
        let authenticated_requester = scoped_principal.expect("checked above");

        if authenticated_requester != intent.requesting_principal {
            let scoped = scoped_request_id.is_some();
            return commit_evaluation_error(
                transaction,
                ErrorInput {
                    authenticated_principal: scoped.then_some(authenticated_requester),
                    request_id: scoped_request_id,
                    intent_hash: current_intent_hash.as_deref(),
                    retry_of_receipt_id: None,
                    stage: "authentication",
                    code: "AUTHENTICATION_FAILED",
                    message: "authenticated requester does not match proposed requester",
                    retryability: Retryability::AfterCondition,
                    required_condition: Some(
                        "authenticated requester matches the proposed requester",
                    ),
                    policy_bundle_id: None,
                    policy_bundle_hash: None,
                    evaluated_at_ms,
                    occupies_slot: scoped,
                },
                &self.config,
            );
        }

        if let Err(validation_error) = intent.validate() {
            let scoped = scoped_request_id.is_some();
            return commit_evaluation_error(
                transaction,
                ErrorInput {
                    authenticated_principal: scoped.then_some(authenticated_requester),
                    request_id: scoped_request_id,
                    intent_hash: None,
                    retry_of_receipt_id: None,
                    stage: "intent_validation",
                    code: "INTENT_INVALID",
                    message: validation_error.message(),
                    retryability: Retryability::Never,
                    required_condition: None,
                    policy_bundle_id: None,
                    policy_bundle_hash: None,
                    evaluated_at_ms,
                    occupies_slot: scoped,
                },
                &self.config,
            );
        }
        let intent_hash = current_intent_hash
            .as_deref()
            .ok_or_else(|| Error::authority("validated intent has no hash"))?;

        if let Some(prior_receipt) = intent.retry_of_receipt_id.as_deref() {
            if !retry_link_is_valid(&transaction, authenticated_requester, prior_receipt)? {
                return commit_evaluation_error(
                    transaction,
                    ErrorInput {
                        authenticated_principal: Some(authenticated_requester),
                        request_id: Some(&intent.request_id),
                        intent_hash: Some(intent_hash),
                        retry_of_receipt_id: None,
                        stage: "retry_validation",
                        code: "ACTION_DATA_AMBIGUOUS",
                        message: "retry link is not an eligible error owned by this requester",
                        retryability: Retryability::Never,
                        required_condition: None,
                        policy_bundle_id: None,
                        policy_bundle_hash: None,
                        evaluated_at_ms,
                        occupies_slot: true,
                    },
                    &self.config,
                );
            }
        }

        let selected =
            match self
                .config
                .policy
                .select(&intent.environment, &intent.tenant, evaluated_at_ms)
            {
                Ok(selected) => selected,
                Err(error) => {
                    return commit_evaluation_error(
                        transaction,
                        ErrorInput {
                            authenticated_principal: Some(authenticated_requester),
                            request_id: Some(&intent.request_id),
                            intent_hash: Some(intent_hash),
                            retry_of_receipt_id: intent.retry_of_receipt_id.as_deref(),
                            stage: "policy_activation",
                            code: error.code(),
                            message: error.message(),
                            retryability: Retryability::AfterCondition,
                            required_condition: Some(
                                "one valid active policy exists for the exact tenant/environment",
                            ),
                            policy_bundle_id: None,
                            policy_bundle_hash: None,
                            evaluated_at_ms,
                            occupies_slot: true,
                        },
                        &self.config,
                    );
                }
            };

        if let Some(reason_code) = self.config.switchboard.refusal_for_intent(intent) {
            return commit_decision(
                transaction,
                DecisionInput {
                    authenticated_requester,
                    intent,
                    intent_hash,
                    policy: selected.bundle,
                    policy_bundle_hash: &selected.policy_bundle_hash,
                    effect: PolicyEffect::deny(reason_code),
                    evaluated_at_ms,
                },
                &self.config,
            );
        }

        let mut effect = selected.bundle.policy.evaluate(intent);
        if let Some(template) = effect.authorization.as_ref() {
            if !self
                .config
                .capabilities
                .covers(&template.capability, &intent.adapter.id)
            {
                let policy_id = effect.policy_id.clone();
                effect = PolicyEffect::deny("POLICY_CAPABILITY_MISMATCH");
                effect.policy_id = policy_id;
            }
        }
        commit_decision(
            transaction,
            DecisionInput {
                authenticated_requester,
                intent,
                intent_hash,
                policy: selected.bundle,
                policy_bundle_hash: &selected.policy_bundle_hash,
                effect,
                evaluated_at_ms,
            },
            &self.config,
        )
    }

    fn claim_trusted_embedding(
        &self,
        authorization_id: &str,
        authenticated_executor: &str,
        executed: &ExecutedAction,
    ) -> Result<ClaimRecord> {
        self.claim_trusted_embedding_at(
            authorization_id,
            authenticated_executor,
            executed,
            now_ms()?,
        )
    }

    pub fn claim_authenticated(
        &self,
        authorization_id: &str,
        executor: &AuthenticatedIdentity,
        executed: &ExecutedAction,
    ) -> Result<ClaimRecord> {
        executor.require_role(LocalRole::Executor)?;
        self.claim_trusted_embedding(authorization_id, executor.principal_id(), executed)
    }

    pub fn claim_authenticated_at(
        &self,
        authorization_id: &str,
        executor: &AuthenticatedIdentity,
        executed: &ExecutedAction,
        claimed_at_ms: i64,
    ) -> Result<ClaimRecord> {
        executor.require_role(LocalRole::Executor)?;
        self.claim_trusted_embedding_at(
            authorization_id,
            executor.principal_id(),
            executed,
            claimed_at_ms,
        )
    }

    fn claim_trusted_embedding_at(
        &self,
        authorization_id: &str,
        authenticated_executor: &str,
        executed: &ExecutedAction,
        claimed_at_ms: i64,
    ) -> Result<ClaimRecord> {
        executed
            .validate()
            .map_err(|_| Error::claim("ACTION_MISMATCH"))?;
        let presented_binding_hash = executed
            .executed_action_hash()
            .map_err(|_| Error::claim("ACTION_MISMATCH"))?;
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        observe_trusted_time(&transaction, claimed_at_ms)?;
        let stored = load_authorization_row(&transaction, authorization_id)?
            .ok_or_else(|| Error::claim("AUTHORIZATION_DENIED"))?;

        match stored.state {
            AuthzState::Claimed => return Err(Error::claim("ALREADY_CLAIMED")),
            AuthzState::Revoked => return Err(Error::claim("AUTHORIZATION_REVOKED")),
            AuthzState::Expired => return Err(Error::claim("AUTHORIZATION_EXPIRED")),
            AuthzState::AuthorizedUnclaimed => {}
        }
        if stored.revoked_at_ms.is_some() {
            return Err(Error::claim("AUTHORIZATION_REVOKED"));
        }
        if authorization_has_active_revocation(&transaction, authorization_id, &stored)? {
            return Err(Error::claim("AUTHORIZATION_REVOKED"));
        }
        verify_stored_authorization_proof(&self.config, authorization_id, &stored)?;
        if claimed_at_ms >= stored.claim_expires_at_ms {
            transaction
                .execute(
                    "UPDATE tlpx_authorizations SET state = 'EXPIRED'
                     WHERE authorization_id = ?1 AND state = 'AUTHORIZED_UNCLAIMED'",
                    [authorization_id],
                )
                .map_err(db_error)?;
            transaction.commit().map_err(db_error)?;
            return Err(Error::claim("AUTHORIZATION_EXPIRED"));
        }
        if stored.executing_principal != authenticated_executor
            || executed.executing_principal != authenticated_executor
        {
            return Err(Error::claim("EXECUTOR_MISMATCH"));
        }
        if let Err(error) = self
            .config
            .switchboard
            .authorize_executor(authenticated_executor, &stored.action)
        {
            return match error.code() {
                "SWITCHBOARD_ACTION_DENIED" => Err(Error::claim("EXECUTOR_ACTION_DENIED")),
                _ => Err(Error::claim("EXECUTOR_NOT_ACTIVE")),
            };
        }
        let active_policy = self
            .config
            .policy
            .select(&stored.environment, &stored.tenant, claimed_at_ms)
            .map_err(|_| Error::claim("POLICY_INACTIVE"))?;
        if stored.policy_bundle_hash != active_policy.policy_bundle_hash {
            return Err(Error::claim("POLICY_INACTIVE"));
        }

        enforce_constraints(
            &self.config.capabilities,
            &executed.binding(),
            &stored.capability,
            &stored.resource_scope,
        )?;
        if stored.action_binding_hash != presented_binding_hash {
            return Err(Error::claim("ACTION_MISMATCH"));
        }

        let claim_id = random_id("claim")?;
        let lease_expires_at_ms = claimed_at_ms
            .checked_add(stored.execution_lease_ms)
            .ok_or_else(|| Error::authority("execution lease overflow"))?;
        let updated = transaction
            .execute(
                "UPDATE tlpx_authorizations
                 SET state = 'CLAIMED', claimed_at_ms = ?1, claim_id = ?2,
                     presented_binding_hash = ?3
                 WHERE authorization_id = ?4
                   AND executing_principal = ?5
                   AND action_binding_hash = ?3
                   AND state = 'AUTHORIZED_UNCLAIMED'
                   AND claim_expires_at_ms > ?1
                   AND revoked_at_ms IS NULL",
                params![
                    claimed_at_ms,
                    claim_id,
                    presented_binding_hash,
                    authorization_id,
                    authenticated_executor,
                ],
            )
            .map_err(db_error)?;
        if updated != 1 {
            return Err(Error::claim("ALREADY_CLAIMED"));
        }

        let sequence = next_sequence(&transaction)?;
        let claim = ClaimRecord {
            claim_id,
            authorization_id: authorization_id.to_string(),
            receipt_id: stored.receipt_id,
            sequence,
            executing_principal: authenticated_executor.to_string(),
            authorized_action_hash: stored.authorized_action_hash,
            action_binding_hash: stored.action_binding_hash,
            executed_action_hash: presented_binding_hash,
            adapter_id: stored.adapter_id,
            adapter_version: stored.adapter_version,
            claimed_at_ms,
            lease_expires_at_ms,
        };
        transaction
            .execute(
                "INSERT INTO tlpx_claims (
                   claim_id, authorization_id, receipt_id, sequence, executing_principal,
                   authorized_action_hash, action_binding_hash, executed_action_hash,
                   adapter_id, adapter_version, claimed_at_ms, lease_expires_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    claim.claim_id,
                    claim.authorization_id,
                    claim.receipt_id,
                    claim.sequence,
                    claim.executing_principal,
                    claim.authorized_action_hash,
                    claim.action_binding_hash,
                    claim.executed_action_hash,
                    claim.adapter_id,
                    claim.adapter_version,
                    claim.claimed_at_ms,
                    claim.lease_expires_at_ms,
                ],
            )
            .map_err(db_error)?;
        let claim_evidence = evidence::claim_record(&claim)?;
        evidence::enqueue(
            &transaction,
            &self.config.evidence,
            claim.sequence,
            0,
            "tlpx.authorization_claim",
            &claim.claim_id,
            &claim_evidence,
        )?;
        transaction.commit().map_err(db_error)?;
        Ok(claim)
    }

    /// Durably opens one execution attempt before a protected side effect may
    /// begin. Only `ExecutionStart::Started` grants this call permission to
    /// start the side effect. An exact retry returns `NotStarted`, including
    /// when the durable attempt itself remains in `STARTED`.
    #[allow(clippy::too_many_arguments)] // Every security-relevant presentation stays explicit.
    pub fn begin_execution_authenticated_at(
        &self,
        claim_id: &str,
        idempotency_key: &str,
        executed: &ExecutedAction,
        executor: &AuthenticatedIdentity,
        adapter: &AuthenticatedAdapterSession,
        adapter_binary_hash: &str,
        started_at_ms: i64,
    ) -> Result<ExecutionStart> {
        executor.require_role(LocalRole::Executor)?;
        let presented_action_hash = executed.executed_action_hash()?;
        assert_hash_string(adapter_binary_hash)
            .map_err(|_| Error::coded("ADAPTER_INTEGRITY_INVALID", "invalid binary hash"))?;
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        observe_trusted_time(&transaction, started_at_ms)?;
        if let Some(existing) = load_execution_by_claim(&transaction, claim_id)? {
            if existing.idempotency_key != idempotency_key
                || existing.executing_principal != executor.principal_id()
                || existing.executed_action_hash != presented_action_hash
                || existing.adapter_principal.as_deref() != Some(adapter.adapter_principal())
                || existing.adapter_binary_hash.as_deref() != Some(adapter_binary_hash)
            {
                return Err(Error::coded(
                    "IDEMPOTENCY_CONFLICT",
                    "execution retry differs from the durable attempt",
                ));
            }
            return Ok(ExecutionStart::NotStarted(existing.lease()));
        }
        let context = load_claim_execution_context(&transaction, claim_id)?
            .ok_or_else(|| Error::coded("EXECUTION_CLAIM_INVALID", "claim does not exist"))?;
        if context.executing_principal != executor.principal_id() {
            return Err(Error::claim("EXECUTOR_MISMATCH"));
        }
        if context.executed_action_hash != presented_action_hash {
            return Err(Error::claim("ACTION_MISMATCH"));
        }
        self.config.adapters.verify(
            &context.adapter_id,
            &context.adapter_version,
            adapter,
            adapter_binary_hash,
            &context.capability,
            &context.action,
        )?;
        let expected_idempotency_key = evidence::authorization_idempotency_key(
            &context.authorization_id,
            &context.executing_principal,
            &context.authorized_action_hash,
        )?;
        if idempotency_key != expected_idempotency_key {
            return Err(Error::coded(
                "IDEMPOTENCY_CONFLICT",
                "execution idempotency key is not bound to this authorization",
            ));
        }
        if started_at_ms < context.claimed_at_ms {
            return Err(Error::coded(
                "EXECUTION_TIME_INVALID",
                "execution cannot start before claim",
            ));
        }
        let (policy_bundle_id, policy_bundle_version) =
            policy_identity_for_hash(&self.config, &context.policy_bundle_hash)?;
        let execution_id = random_id("execution")?;
        let late = started_at_ms >= context.lease_expires_at_ms;
        let stored_started_at_ms = if late {
            context.claimed_at_ms
        } else {
            started_at_ms
        };
        transaction
            .execute(
                "INSERT INTO tlpx_executions (
                   execution_id, claim_id, authorization_id, receipt_id, idempotency_key,
                   requesting_principal, executing_principal, intent_hash,
                   authorized_action_hash, executed_action_hash, target,
                   policy_bundle_id, policy_bundle_version, policy_bundle_hash,
                   adapter_id, adapter_version, adapter_principal, adapter_binary_hash,
                   started_at_ms, lease_expires_at_ms, state,
                   outcome_unknown_at_ms, reconciliation_required_at_ms,
                   terminal_sequence, ended_at_ms, result_summary, result_hash,
                   external_evidence_reference, cancellation_outcome
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                   ?15, ?16, ?17, ?18, ?19, ?20, ?21, NULL, NULL, ?22, ?23, ?24, NULL, NULL, NULL
                 )",
                params![
                    execution_id,
                    context.claim_id,
                    context.authorization_id,
                    context.receipt_id,
                    idempotency_key,
                    context.requesting_principal,
                    context.executing_principal,
                    context.intent_hash,
                    context.authorized_action_hash,
                    context.executed_action_hash,
                    context.target,
                    policy_bundle_id,
                    policy_bundle_version,
                    context.policy_bundle_hash,
                    context.adapter_id,
                    context.adapter_version,
                    adapter.adapter_principal(),
                    adapter_binary_hash,
                    stored_started_at_ms,
                    context.lease_expires_at_ms,
                    if late { "LEASE_EXPIRED" } else { "STARTED" },
                    if late {
                        Some(next_sequence(&transaction)?)
                    } else {
                        None
                    },
                    if late { Some(started_at_ms) } else { None },
                    if late {
                        Some("claim lease expired before protected execution began")
                    } else {
                        None
                    },
                ],
            )
            .map_err(db_error)?;
        let stored = load_execution_by_id(&transaction, &execution_id)?
            .ok_or_else(|| Error::authority("inserted execution is missing"))?;
        if late {
            let receipt = stored.receipt()?;
            let record = evidence::execution_record(&receipt)?;
            evidence::enqueue(
                &transaction,
                &self.config.evidence,
                receipt.sequence,
                0,
                "tlpx.execution",
                &receipt.execution_id,
                &record,
            )?;
        }
        transaction.commit().map_err(db_error)?;
        if late {
            Ok(ExecutionStart::NotStarted(stored.lease()))
        } else {
            Ok(ExecutionStart::Started(stored.lease()))
        }
    }

    pub fn finish_execution_authenticated_at(
        &self,
        execution_id: &str,
        executor: &AuthenticatedIdentity,
        terminal_state: ExecutionState,
        result: ExecutionResultEvidence,
        cancellation_outcome: Option<CancellationOutcome>,
        ended_at_ms: i64,
    ) -> Result<ExecutionReceipt> {
        executor.require_role(LocalRole::Executor)?;
        if !matches!(
            terminal_state,
            ExecutionState::Completed | ExecutionState::Failed | ExecutionState::Cancelled
        ) {
            return Err(Error::coded(
                "EXECUTION_STATE_INVALID",
                "executor may finish only as COMPLETED, FAILED, or CANCELLED",
            ));
        }
        validate_terminal_execution_input(terminal_state, &result, cancellation_outcome)?;
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        observe_trusted_time(&transaction, ended_at_ms)?;
        let stored = load_execution_by_id(&transaction, execution_id)?
            .ok_or_else(|| Error::coded("EXECUTION_NOT_FOUND", "execution does not exist"))?;
        if stored.executing_principal != executor.principal_id() {
            return Err(Error::claim("EXECUTOR_MISMATCH"));
        }
        if stored.state.is_terminal() {
            let receipt = stored.receipt()?;
            if receipt.state == terminal_state
                && receipt.result == result
                && receipt.cancellation_outcome == cancellation_outcome
                && receipt.ended_at_ms == ended_at_ms
            {
                return Ok(receipt);
            }
            return Err(Error::coded(
                "IDEMPOTENCY_CONFLICT",
                "terminal execution retry differs from the durable receipt",
            ));
        }
        if stored.state != ExecutionState::Started {
            return Err(Error::coded(
                "RECONCILIATION_REQUIRED",
                "unknown execution outcome must be reconciled, not directly finished",
            ));
        }
        let receipt = finalize_execution(
            &transaction,
            &self.config,
            stored,
            terminal_state,
            result,
            cancellation_outcome,
            ended_at_ms,
        )?;
        transaction.commit().map_err(db_error)?;
        Ok(receipt)
    }

    pub fn mark_execution_outcome_unknown_authenticated_at(
        &self,
        execution_id: &str,
        actor: &AuthenticatedIdentity,
        observed_at_ms: i64,
    ) -> Result<ExecutionState> {
        if !actor.has_role(LocalRole::Executor) && !actor.has_role(LocalRole::Reconciler) {
            return Err(Error::coded(
                "AUTHENTICATION_FAILED",
                "authenticated local principal lacks executor or reconciler role",
            ));
        }
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        observe_trusted_time(&transaction, observed_at_ms)?;
        let stored = load_execution_by_id(&transaction, execution_id)?
            .ok_or_else(|| Error::coded("EXECUTION_NOT_FOUND", "execution does not exist"))?;
        if actor.has_role(LocalRole::Executor)
            && stored.executing_principal != actor.principal_id()
            && !actor.has_role(LocalRole::Reconciler)
        {
            return Err(Error::claim("EXECUTOR_MISMATCH"));
        }
        match stored.state {
            ExecutionState::Started => {
                if observed_at_ms < stored.started_at_ms {
                    return Err(Error::coded(
                        "EXECUTION_TIME_INVALID",
                        "unknown outcome cannot precede execution start",
                    ));
                }
                transaction
                    .execute(
                        "UPDATE tlpx_executions
                         SET state = 'EXECUTION_OUTCOME_UNKNOWN', outcome_unknown_at_ms = ?1
                         WHERE execution_id = ?2 AND state = 'STARTED'",
                        params![observed_at_ms, execution_id],
                    )
                    .map_err(db_error)?;
                transaction.commit().map_err(db_error)?;
                Ok(ExecutionState::ExecutionOutcomeUnknown)
            }
            ExecutionState::ExecutionOutcomeUnknown | ExecutionState::ReconciliationRequired => {
                Ok(stored.state)
            }
            _ => Err(Error::coded(
                "EXECUTION_TERMINAL",
                "terminal execution cannot become unknown",
            )),
        }
    }

    pub fn require_reconciliation_authenticated_at(
        &self,
        execution_id: &str,
        reconciler: &AuthenticatedIdentity,
        required_at_ms: i64,
    ) -> Result<ExecutionState> {
        reconciler.require_role(LocalRole::Reconciler)?;
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        observe_trusted_time(&transaction, required_at_ms)?;
        let stored = load_execution_by_id(&transaction, execution_id)?
            .ok_or_else(|| Error::coded("EXECUTION_NOT_FOUND", "execution does not exist"))?;
        match stored.state {
            ExecutionState::ExecutionOutcomeUnknown => {
                if required_at_ms < stored.outcome_unknown_at_ms.unwrap_or(stored.started_at_ms) {
                    return Err(Error::coded(
                        "EXECUTION_TIME_INVALID",
                        "reconciliation cannot precede unknown outcome",
                    ));
                }
                transaction
                    .execute(
                        "UPDATE tlpx_executions
                         SET state = 'RECONCILIATION_REQUIRED', reconciliation_required_at_ms = ?1
                         WHERE execution_id = ?2 AND state = 'EXECUTION_OUTCOME_UNKNOWN'",
                        params![required_at_ms, execution_id],
                    )
                    .map_err(db_error)?;
                transaction.commit().map_err(db_error)?;
                Ok(ExecutionState::ReconciliationRequired)
            }
            ExecutionState::ReconciliationRequired => Ok(stored.state),
            ExecutionState::Started => Err(Error::coded(
                "EXECUTION_OUTCOME_NOT_UNKNOWN",
                "execution must first be marked outcome unknown",
            )),
            _ => Err(Error::coded(
                "EXECUTION_TERMINAL",
                "terminal execution cannot require reconciliation",
            )),
        }
    }

    pub fn reconcile_execution_authenticated_at(
        &self,
        execution_id: &str,
        reconciler: &AuthenticatedIdentity,
        terminal_state: ExecutionState,
        result: ExecutionResultEvidence,
        ended_at_ms: i64,
    ) -> Result<ExecutionReceipt> {
        reconciler.require_role(LocalRole::Reconciler)?;
        if !matches!(
            terminal_state,
            ExecutionState::CompletedConfirmed
                | ExecutionState::FailedConfirmed
                | ExecutionState::OutcomeUnknownFinal
        ) {
            return Err(Error::coded(
                "EXECUTION_STATE_INVALID",
                "reconciliation must choose a confirmed or honestly unknown terminal state",
            ));
        }
        validate_terminal_execution_input(terminal_state, &result, None)?;
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        observe_trusted_time(&transaction, ended_at_ms)?;
        let stored = load_execution_by_id(&transaction, execution_id)?
            .ok_or_else(|| Error::coded("EXECUTION_NOT_FOUND", "execution does not exist"))?;
        if stored.state.is_terminal() {
            let receipt = stored.receipt()?;
            if receipt.state == terminal_state
                && receipt.result == result
                && receipt.ended_at_ms == ended_at_ms
            {
                return Ok(receipt);
            }
            return Err(Error::coded(
                "IDEMPOTENCY_CONFLICT",
                "reconciliation retry differs from the durable receipt",
            ));
        }
        if stored.state != ExecutionState::ReconciliationRequired {
            return Err(Error::coded(
                "RECONCILIATION_NOT_READY",
                "execution is not awaiting reconciliation",
            ));
        }
        if ended_at_ms
            < stored
                .reconciliation_required_at_ms
                .unwrap_or(stored.started_at_ms)
        {
            return Err(Error::coded(
                "EXECUTION_TIME_INVALID",
                "reconciliation result cannot precede the reconciliation requirement",
            ));
        }
        let receipt = finalize_execution(
            &transaction,
            &self.config,
            stored,
            terminal_state,
            result,
            None,
            ended_at_ms,
        )?;
        transaction.commit().map_err(db_error)?;
        Ok(receipt)
    }

    /// Recovers an expired claimed execution. If no protected execution began,
    /// this emits a terminal LEASE_EXPIRED receipt. If one began, it enters the
    /// unknown-outcome process and must be reconciled without replay.
    pub fn recover_expired_claim_authenticated_at(
        &self,
        reconciler: &AuthenticatedIdentity,
        claim_id: &str,
        recovered_at_ms: i64,
    ) -> Result<ExecutionState> {
        reconciler.require_role(LocalRole::Reconciler)?;
        self.recover_expired_claim_at(claim_id, recovered_at_ms)
    }

    fn recover_expired_claim_at(
        &self,
        claim_id: &str,
        recovered_at_ms: i64,
    ) -> Result<ExecutionState> {
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        observe_trusted_time(&transaction, recovered_at_ms)?;
        if let Some(stored) = load_execution_by_claim(&transaction, claim_id)? {
            if recovered_at_ms < stored.lease_expires_at_ms {
                return Err(Error::coded(
                    "EXECUTION_LEASE_ACTIVE",
                    "execution lease has not expired",
                ));
            }
            return match stored.state {
                ExecutionState::Started => {
                    transaction
                        .execute(
                            "UPDATE tlpx_executions
                             SET state = 'EXECUTION_OUTCOME_UNKNOWN', outcome_unknown_at_ms = ?1
                             WHERE execution_id = ?2 AND state = 'STARTED'",
                            params![recovered_at_ms, stored.execution_id],
                        )
                        .map_err(db_error)?;
                    transaction.commit().map_err(db_error)?;
                    Ok(ExecutionState::ExecutionOutcomeUnknown)
                }
                _ => Ok(stored.state),
            };
        }
        let context = load_claim_execution_context(&transaction, claim_id)?
            .ok_or_else(|| Error::coded("EXECUTION_CLAIM_INVALID", "claim does not exist"))?;
        if recovered_at_ms < context.lease_expires_at_ms {
            return Err(Error::coded(
                "EXECUTION_LEASE_ACTIVE",
                "claim execution lease has not expired",
            ));
        }
        let idempotency_key = evidence::authorization_idempotency_key(
            &context.authorization_id,
            &context.executing_principal,
            &context.authorized_action_hash,
        )?;
        let (policy_bundle_id, policy_bundle_version) =
            policy_identity_for_hash(&self.config, &context.policy_bundle_hash)?;
        let execution_id = random_id("execution")?;
        let sequence = next_sequence(&transaction)?;
        transaction
            .execute(
                "INSERT INTO tlpx_executions (
                   execution_id, claim_id, authorization_id, receipt_id, idempotency_key,
                   requesting_principal, executing_principal, intent_hash,
                   authorized_action_hash, executed_action_hash, target,
                   policy_bundle_id, policy_bundle_version, policy_bundle_hash,
                   adapter_id, adapter_version, adapter_principal, adapter_binary_hash,
                   started_at_ms, lease_expires_at_ms, state,
                   outcome_unknown_at_ms, reconciliation_required_at_ms,
                   terminal_sequence, ended_at_ms, result_summary, result_hash,
                   external_evidence_reference, cancellation_outcome
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                   ?15, ?16, NULL, NULL, ?17, ?18, 'LEASE_EXPIRED', NULL, NULL,
                   ?19, ?20, ?21, NULL, NULL, NULL
                 )",
                params![
                    execution_id,
                    context.claim_id,
                    context.authorization_id,
                    context.receipt_id,
                    idempotency_key,
                    context.requesting_principal,
                    context.executing_principal,
                    context.intent_hash,
                    context.authorized_action_hash,
                    context.executed_action_hash,
                    context.target,
                    policy_bundle_id,
                    policy_bundle_version,
                    context.policy_bundle_hash,
                    context.adapter_id,
                    context.adapter_version,
                    context.claimed_at_ms,
                    context.lease_expires_at_ms,
                    sequence,
                    recovered_at_ms,
                    "claim lease expired before protected execution began",
                ],
            )
            .map_err(db_error)?;
        let receipt = load_execution_by_id(&transaction, &execution_id)?
            .ok_or_else(|| Error::authority("inserted execution is missing"))?
            .receipt()?;
        let record = evidence::execution_record(&receipt)?;
        evidence::enqueue(
            &transaction,
            &self.config.evidence,
            receipt.sequence,
            0,
            "tlpx.execution",
            &receipt.execution_id,
            &record,
        )?;
        transaction.commit().map_err(db_error)?;
        Ok(ExecutionState::LeaseExpired)
    }

    pub fn execution_state(&self, execution_id: &str) -> Result<Option<ExecutionState>> {
        let connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        connection
            .query_row(
                "SELECT state FROM tlpx_executions WHERE execution_id = ?1",
                [execution_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(db_error)?
            .map(|state| ExecutionState::parse(&state))
            .transpose()
    }

    pub fn revoke_authenticated(
        &self,
        revoker: &AuthenticatedIdentity,
        scope: RevocationScope,
        scope_id: &str,
        reason: RevocationReason,
    ) -> Result<RevocationRecord> {
        self.revoke_authenticated_at(revoker, scope, scope_id, reason, now_ms()?)
    }

    pub fn revoke_authenticated_at(
        &self,
        revoker: &AuthenticatedIdentity,
        scope: RevocationScope,
        scope_id: &str,
        reason: RevocationReason,
        revoked_at_ms: i64,
    ) -> Result<RevocationRecord> {
        revoker.require_role(LocalRole::EmergencyCanceller)?;
        self.revoke_scope_at(
            revoker.principal_id(),
            scope,
            scope_id,
            reason,
            revoked_at_ms,
        )
    }

    fn revoke_scope_at(
        &self,
        revoking_principal: &str,
        scope: RevocationScope,
        scope_id: &str,
        reason: RevocationReason,
        revoked_at_ms: i64,
    ) -> Result<RevocationRecord> {
        if revoking_principal.is_empty() || scope_id.is_empty() || revoked_at_ms < 0 {
            return Err(Error::coded(
                "REVOCATION_INVALID",
                "revocation actor, scope id, and timestamp must be valid",
            ));
        }
        if scope == RevocationScope::SigningKey {
            if self.config.evidence.keys.purpose_for(scope_id).is_none() {
                return Err(Error::coded(
                    "REVOCATION_INVALID",
                    "signing-key revocation must name a configured role key",
                ));
            }
            if !matches!(
                reason,
                RevocationReason::SigningKeyCompromised | RevocationReason::EmergencyDeny
            ) {
                return Err(Error::coded(
                    "REVOCATION_INVALID",
                    "signing-key revocation requires a key-compromise or emergency reason",
                ));
            }
        }
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        observe_trusted_time(&transaction, revoked_at_ms)?;
        let already_active = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM tlpx_revocations WHERE scope_type = ?1 AND scope_id = ?2
                 )",
                params![scope.as_str(), scope_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(db_error)?;
        if already_active {
            return Err(Error::coded(
                "REVOCATION_ALREADY_ACTIVE",
                "the exact revocation scope is already active",
            ));
        }
        if scope == RevocationScope::Authorization {
            let updated = transaction
                .execute(
                    "UPDATE tlpx_authorizations SET state = 'REVOKED', revoked_at_ms = ?1
                     WHERE authorization_id = ?2 AND state = 'AUTHORIZED_UNCLAIMED'
                       AND revoked_at_ms IS NULL",
                    params![revoked_at_ms, scope_id],
                )
                .map_err(db_error)?;
            if updated != 1 {
                return Err(Error::claim("AUTHORIZATION_TERMINAL"));
            }
        }
        let sequence = next_sequence(&transaction)?;
        let record = RevocationRecord {
            revocation_id: random_id("revocation")?,
            sequence,
            scope,
            scope_id: scope_id.to_string(),
            revoking_principal: revoking_principal.to_string(),
            reason,
            revoked_at_ms,
        };
        transaction
            .execute(
                "INSERT INTO tlpx_revocations (
                   revocation_id, sequence, scope_type, scope_id,
                   revoking_principal, reason, revoked_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    record.revocation_id,
                    record.sequence,
                    record.scope.as_str(),
                    record.scope_id,
                    record.revoking_principal,
                    record.reason.as_str(),
                    record.revoked_at_ms,
                ],
            )
            .map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(record)
    }

    pub fn cancel_pending_authenticated(
        &self,
        receipt_id: &str,
        canceller: &AuthenticatedIdentity,
        reason: CancellationReason,
    ) -> Result<CancellationRecord> {
        self.cancel_pending_authenticated_at(receipt_id, canceller, reason, now_ms()?)
    }

    pub fn cancel_pending_authenticated_at(
        &self,
        receipt_id: &str,
        canceller: &AuthenticatedIdentity,
        reason: CancellationReason,
        cancelled_at_ms: i64,
    ) -> Result<CancellationRecord> {
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        let pending = transaction
            .query_row(
                "SELECT requesting_principal, policy_bundle_hash, state, approval_expires_at_ms
                 FROM tlpx_pending_approvals WHERE receipt_id = ?1",
                [receipt_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(db_error)?
            .ok_or_else(|| Error::coded("APPROVAL_TERMINAL", "pending approval does not exist"))?;
        if pending.2 != "PENDING_APPROVAL" {
            return Err(Error::coded(
                "APPROVAL_TERMINAL",
                "pending approval already has a terminal outcome",
            ));
        }
        observe_trusted_time(&transaction, cancelled_at_ms)?;
        if cancelled_at_ms >= pending.3 {
            expire_pending_transaction(&transaction, receipt_id, cancelled_at_ms)?;
            transaction.commit().map_err(db_error)?;
            return Err(Error::coded(
                "APPROVAL_TERMINAL",
                "pending approval expired before cancellation",
            ));
        }
        let role = cancellation_role(&transaction, receipt_id, &pending.0, canceller, reason)?;
        let sequence = next_sequence(&transaction)?;
        let updated = transaction
            .execute(
                "UPDATE tlpx_pending_approvals
                 SET state = 'CANCELLED', terminal_sequence = ?1, terminal_at_ms = ?2,
                     terminal_actor = ?3, terminal_actor_type = ?4,
                     terminal_role = ?5, terminal_reason = ?6
                 WHERE receipt_id = ?7 AND state = 'PENDING_APPROVAL'",
                params![
                    sequence,
                    cancelled_at_ms,
                    canceller.principal_id(),
                    canceller.party_type().as_str(),
                    role.as_str(),
                    reason.as_str(),
                    receipt_id,
                ],
            )
            .map_err(db_error)?;
        if updated != 1 {
            return Err(Error::coded(
                "APPROVAL_TERMINAL",
                "pending approval already has a terminal outcome",
            ));
        }
        transaction
            .execute(
                "INSERT INTO tlpx_operator_actions (
                   receipt_id, sequence, outcome, actor_id, actor_type, actor_role,
                   reason, policy_bundle_hash, acted_at_ms
                 ) VALUES (?1, ?2, 'CANCEL', ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    receipt_id,
                    sequence,
                    canceller.principal_id(),
                    canceller.party_type().as_str(),
                    role.as_str(),
                    reason.as_str(),
                    pending.1,
                    cancelled_at_ms,
                ],
            )
            .map_err(db_error)?;
        let record = CancellationRecord {
            receipt_id: receipt_id.to_string(),
            sequence,
            canceller: canceller.principal_id().to_string(),
            canceller_role: role,
            reason,
            cancelled_at_ms,
            policy_bundle_hash: pending.1,
        };
        let operator_evidence = evidence::cancellation_record(CancellationEvidenceInput {
            record: &record,
            party_type: canceller.party_type(),
        })?;
        evidence::enqueue(
            &transaction,
            &self.config.evidence,
            sequence,
            0,
            "tlpx.operator_action",
            receipt_id,
            &operator_evidence,
        )?;
        transaction.commit().map_err(db_error)?;
        Ok(record)
    }

    pub fn pending_approval_authenticated(
        &self,
        receipt_id: &str,
        operator: &AuthenticatedIdentity,
    ) -> Result<PendingApprovalView> {
        self.pending_approval_authenticated_at(receipt_id, operator, now_ms()?)
    }

    pub fn pending_approval_authenticated_at(
        &self,
        receipt_id: &str,
        operator: &AuthenticatedIdentity,
        viewed_at_ms: i64,
    ) -> Result<PendingApprovalView> {
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        let pending = load_pending_authorization(&transaction, receipt_id)?
            .ok_or_else(|| Error::coded("APPROVAL_TERMINAL", "pending approval does not exist"))?;
        if pending.state != ApprovalState::Pending {
            return Err(Error::coded(
                "APPROVAL_TERMINAL",
                "pending approval already has a terminal outcome",
            ));
        }
        observe_trusted_time(&transaction, viewed_at_ms)?;
        if viewed_at_ms >= pending.authorization.approval_expires_at_ms {
            expire_pending_transaction(&transaction, receipt_id, viewed_at_ms)?;
            transaction.commit().map_err(db_error)?;
            return Err(Error::coded(
                "APPROVAL_EXPIRED",
                "pending approval window expired",
            ));
        }
        require_approval_operator(operator, &pending.approval_route)?;
        let view = pending.view();
        transaction.commit().map_err(db_error)?;
        Ok(view)
    }

    pub fn resolve_pending_authenticated(
        &self,
        receipt_id: &str,
        operator: &AuthenticatedIdentity,
        outcome: ApprovalOutcome,
        presentation: ApprovalPresentation,
    ) -> Result<ApprovalResolution> {
        self.resolve_pending_authenticated_at(
            receipt_id,
            operator,
            outcome,
            presentation,
            now_ms()?,
        )
    }

    pub fn resolve_pending_authenticated_at(
        &self,
        receipt_id: &str,
        operator: &AuthenticatedIdentity,
        outcome: ApprovalOutcome,
        presentation: ApprovalPresentation,
        acted_at_ms: i64,
    ) -> Result<ApprovalResolution> {
        validate_approval_presentation(&presentation)?;
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        let pending = load_pending_authorization(&transaction, receipt_id)?
            .ok_or_else(|| Error::coded("APPROVAL_TERMINAL", "pending approval does not exist"))?;
        if pending.state != ApprovalState::Pending {
            return Err(Error::coded(
                "APPROVAL_TERMINAL",
                "pending approval already has a terminal outcome",
            ));
        }
        observe_trusted_time(&transaction, acted_at_ms)?;
        if acted_at_ms >= pending.authorization.approval_expires_at_ms {
            expire_pending_transaction(&transaction, receipt_id, acted_at_ms)?;
            transaction.commit().map_err(db_error)?;
            return Err(Error::coded(
                "APPROVAL_EXPIRED",
                "pending approval window expired",
            ));
        }
        require_approval_operator(operator, &pending.approval_route)?;
        if presentation.authorized_action_hash != pending.authorization.authorized_action_hash {
            return Err(Error::coded(
                "APPROVAL_PRESENTATION_MISMATCH",
                "operator presentation does not bind the pending authorized action",
            ));
        }

        let authorization = if outcome == ApprovalOutcome::Approve {
            let selected = self.config.policy.select(
                &pending.authorization.environment,
                &pending.authorization.tenant,
                acted_at_ms,
            )?;
            if selected.policy_bundle_hash != pending.authorization.policy_bundle_hash {
                return Err(Error::coded(
                    "POLICY_INACTIVE",
                    "pending approval policy is no longer active",
                ));
            }
            self.config.switchboard.authorize_executor(
                &pending.authorization.executing_principal,
                &pending.authorization.action,
            )?;
            let issued =
                issue_pending_authorization(&self.config, &pending.authorization, acted_at_ms)?;
            if signing_key_is_durably_revoked(&transaction, &self.config)? {
                return Err(Error::coded(
                    "KEY_REVOKED",
                    "active authorization signing key is durably revoked",
                ));
            }
            Some(issued)
        } else {
            None
        };
        let sequence = next_sequence(&transaction)?;
        let updated = transaction
            .execute(
                "UPDATE tlpx_pending_approvals
                 SET state = ?1, terminal_sequence = ?2, terminal_at_ms = ?3,
                     terminal_actor = ?4, terminal_actor_type = 'human',
                     terminal_role = 'OPERATOR', terminal_reason = NULL
                 WHERE receipt_id = ?5 AND state = 'PENDING_APPROVAL'",
                params![
                    outcome.terminal_state().as_str(),
                    sequence,
                    acted_at_ms,
                    operator.principal_id(),
                    receipt_id,
                ],
            )
            .map_err(db_error)?;
        if updated != 1 {
            return Err(Error::coded(
                "APPROVAL_TERMINAL",
                "pending approval already has a terminal outcome",
            ));
        }
        transaction
            .execute(
                "INSERT INTO tlpx_operator_actions (
                   receipt_id, sequence, outcome, actor_id, actor_type, actor_role,
                   reason, policy_bundle_hash, acted_at_ms, authorized_action_hash,
                   renderer_id, renderer_version
                 ) VALUES (?1, ?2, ?3, ?4, 'human', 'OPERATOR', NULL, ?5, ?6, ?7, ?8, ?9)",
                params![
                    receipt_id,
                    sequence,
                    outcome.as_str(),
                    operator.principal_id(),
                    pending.authorization.policy_bundle_hash,
                    acted_at_ms,
                    pending.authorization.authorized_action_hash,
                    presentation.renderer_id,
                    presentation.renderer_version,
                ],
            )
            .map_err(db_error)?;
        if let Some(ref issued) = authorization {
            insert_authorization(&transaction, issued)?;
            let evaluation_updated = transaction
                .execute(
                    "UPDATE tlpx_evaluations SET authorization_id = ?1
                     WHERE receipt_id = ?2 AND decision = 'REQUIRE_APPROVAL'
                       AND authorization_id IS NULL",
                    params![issued.authorization_id, receipt_id],
                )
                .map_err(db_error)?;
            if evaluation_updated != 1 {
                return Err(Error::authority(
                    "approved authorization could not bind its decision",
                ));
            }
        }
        let resolution = ApprovalResolution {
            receipt_id: receipt_id.to_string(),
            sequence,
            outcome,
            operator: operator.principal_id().to_string(),
            acted_at_ms,
            authorized_action_hash: pending.authorization.authorized_action_hash.clone(),
            policy_bundle_hash: pending.authorization.policy_bundle_hash.clone(),
            approval_route: pending.approval_route.clone(),
            renderer_id: presentation.renderer_id,
            renderer_version: presentation.renderer_version,
            authorization,
        };
        let operator_evidence = evidence::approval_record(ApprovalEvidenceInput {
            record: &resolution,
        })?;
        evidence::enqueue(
            &transaction,
            &self.config.evidence,
            sequence,
            0,
            "tlpx.operator_action",
            receipt_id,
            &operator_evidence,
        )?;
        if let Some(ref issued) = resolution.authorization {
            let authorization_evidence = evidence::authorization_record(issued)?;
            evidence::enqueue(
                &transaction,
                &self.config.evidence,
                sequence,
                1,
                "tlpx.authorization",
                &issued.authorization_id,
                &authorization_evidence,
            )?;
        }
        transaction.commit().map_err(db_error)?;
        Ok(resolution)
    }

    pub fn expire_pending_authenticated_at(
        &self,
        system_authority: &AuthenticatedIdentity,
        receipt_id: &str,
        expired_at_ms: i64,
    ) -> Result<i64> {
        system_authority.require_role(LocalRole::Authority)?;
        self.expire_pending_at(receipt_id, expired_at_ms)
    }

    fn expire_pending_at(&self, receipt_id: &str, expired_at_ms: i64) -> Result<i64> {
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        evidence::reconcile(&transaction, &self.config.evidence)?;
        let pending = load_pending_authorization(&transaction, receipt_id)?
            .ok_or_else(|| Error::coded("APPROVAL_TERMINAL", "pending approval does not exist"))?;
        if pending.state != ApprovalState::Pending {
            return Err(Error::coded(
                "APPROVAL_TERMINAL",
                "pending approval already has a terminal outcome",
            ));
        }
        observe_trusted_time(&transaction, expired_at_ms)?;
        if expired_at_ms < pending.authorization.approval_expires_at_ms {
            return Err(Error::coded(
                "APPROVAL_NOT_EXPIRED",
                "pending approval window is still open",
            ));
        }
        let sequence = expire_pending_transaction(&transaction, receipt_id, expired_at_ms)?;
        transaction.commit().map_err(db_error)?;
        Ok(sequence)
    }

    pub fn approval_state(&self, receipt_id: &str) -> Result<Option<ApprovalState>> {
        let connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        connection
            .query_row(
                "SELECT state FROM tlpx_pending_approvals WHERE receipt_id = ?1",
                [receipt_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(db_error)?
            .map(|state| ApprovalState::parse(&state))
            .transpose()
    }

    pub fn state(&self, authorization_id: &str) -> Result<Option<AuthzState>> {
        let connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        connection
            .query_row(
                "SELECT state FROM tlpx_authorizations WHERE authorization_id = ?1",
                [authorization_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(db_error)?
            .map(|value| AuthzState::parse(&value))
            .transpose()
    }

    pub fn evaluation_event_count(&self) -> Result<i64> {
        let connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        connection
            .query_row("SELECT COUNT(*) FROM tlpx_evaluations", [], |row| {
                row.get(0)
            })
            .map_err(db_error)
    }

    /// Returns sealed, canonical records that have not yet been acknowledged by an exporter.
    pub fn pending_evidence(&self, limit: usize) -> Result<Vec<SealedEvidence>> {
        let connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        evidence::reconcile(&connection, &self.config.evidence)?;
        evidence::pending(&connection, limit)
    }

    /// Internal acknowledgement used only after a durable exporter has synced
    /// the corresponding row. Repeating the same acknowledgement is safe.
    pub(crate) fn mark_evidence_exported_at(
        &self,
        outbox_id: i64,
        expected_chain_hash: &str,
        exported_at_ms: i64,
    ) -> Result<bool> {
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        evidence::mark_exported(
            &mut connection,
            &self.config.evidence,
            outbox_id,
            expected_chain_hash,
            exported_at_ms,
        )
    }

    pub(crate) fn evidence_snapshot(&self) -> Result<Vec<SealedEvidence>> {
        let connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        evidence::reconcile(&connection, &self.config.evidence)?;
        evidence::all(&connection)
    }

    /// Verifies canonical payloads, record/chain hashes, HMAC seals, source
    /// rows, and complete authority-to-outbox coverage.
    pub fn reconcile_evidence(&self) -> Result<EvidenceReconciliation> {
        let connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        evidence::reconcile(&connection, &self.config.evidence)
    }

    /// Returns bounded aggregate health signals after SQLite and evidence
    /// integrity checks. It exposes no principals, action data, or key bytes.
    pub fn operational_snapshot(&self) -> Result<OperationalSnapshot> {
        let mut connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(db_error)?;
        let quick_check = transaction
            .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
            .map_err(db_error)?;
        if quick_check != "ok" {
            return Err(Error::coded(
                "OPERATIONAL_INTEGRITY_FAILED",
                "SQLite quick_check did not return ok",
            ));
        }
        let foreign_key_violation = transaction
            .prepare("PRAGMA foreign_key_check")
            .map_err(db_error)?
            .query([])
            .map_err(db_error)?
            .next()
            .map_err(db_error)?
            .is_some();
        if foreign_key_violation {
            return Err(Error::coded(
                "OPERATIONAL_INTEGRITY_FAILED",
                "SQLite foreign-key integrity check failed",
            ));
        }
        let evidence = evidence::reconcile(&transaction, &self.config.evidence)?;
        let trusted_time_last_ms = transaction
            .query_row(
                "SELECT last_observed_ms FROM tlpx_trusted_time WHERE singleton = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(db_error)?;
        let counts = transaction
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM tlpx_pending_approvals
                    WHERE state = 'PENDING_APPROVAL'),
                   (SELECT COUNT(*) FROM tlpx_authorizations
                    WHERE state = 'AUTHORIZED_UNCLAIMED'),
                   (SELECT COUNT(*) FROM tlpx_executions WHERE state = 'STARTED'),
                   (SELECT COUNT(*) FROM tlpx_executions
                    WHERE state = 'EXECUTION_OUTCOME_UNKNOWN'),
                   (SELECT COUNT(*) FROM tlpx_executions
                    WHERE state = 'RECONCILIATION_REQUIRED'),
                   (SELECT COUNT(*) FROM tlpx_revocations)",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(OperationalSnapshot {
            trusted_time_last_ms,
            evidence,
            pending_approvals: counts.0,
            unclaimed_authorizations: counts.1,
            started_executions: counts.2,
            outcome_unknown_executions: counts.3,
            reconciliation_required_executions: counts.4,
            active_revocations: counts.5,
        })
    }
}

pub fn enforce_constraints(
    capabilities: &CapabilityRegistry,
    binding: &ActionBinding,
    capability: &str,
    resource_scope: &[String],
) -> Result<()> {
    if !resource_scope.iter().any(|scope| scope == &binding.target) {
        return Err(Error::claim("AUTHORIZATION_SCOPE_DENIED"));
    }
    if !capabilities.covers(capability, &binding.adapter.id) {
        return Err(Error::claim("AUTHORIZATION_CAPABILITY_DENIED"));
    }
    Ok(())
}

fn cancellation_role(
    transaction: &Transaction<'_>,
    receipt_id: &str,
    requesting_principal: &str,
    identity: &AuthenticatedIdentity,
    reason: CancellationReason,
) -> Result<CancellationRole> {
    if reason == CancellationReason::RequesterWithdrawn
        && identity.has_role(LocalRole::Requester)
        && identity.principal_id() == requesting_principal
    {
        return Ok(CancellationRole::Requester);
    }
    if reason == CancellationReason::OperatorCancelled && identity.has_role(LocalRole::Operator) {
        let mut statement = transaction
            .prepare(
                "SELECT route_id FROM tlpx_pending_approval_routes
                 WHERE receipt_id = ?1 ORDER BY position",
            )
            .map_err(db_error)?;
        let routes = statement
            .query_map([receipt_id], |row| row.get::<_, String>(0))
            .map_err(db_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error)?;
        if routes
            .iter()
            .any(|route| identity.permits_approval_route(route))
        {
            return Ok(CancellationRole::Operator);
        }
    }
    if reason == CancellationReason::EmergencyRevocation
        && identity.has_role(LocalRole::EmergencyCanceller)
    {
        return Ok(CancellationRole::EmergencyAuthority);
    }
    Err(Error::coded(
        "CANCELLATION_UNAUTHORIZED",
        "authenticated principal is not authorized to cancel this approval",
    ))
}

#[derive(Debug, Clone)]
struct StoredPendingApproval {
    authorization: PendingAuthorization,
    state: ApprovalState,
    approval_route: Vec<String>,
}

impl StoredPendingApproval {
    fn view(&self) -> PendingApprovalView {
        PendingApprovalView {
            receipt_id: self.authorization.receipt_id.clone(),
            requesting_principal: self.authorization.requesting_principal.clone(),
            executing_principal: self.authorization.executing_principal.clone(),
            action: self.authorization.action.clone(),
            target: self.authorization.target.clone(),
            authorized_action_json: self.authorization.authorized_action_json.clone(),
            authorized_action_hash: self.authorization.authorized_action_hash.clone(),
            action_binding_hash: self.authorization.action_binding_hash.clone(),
            approval_route: self.approval_route.clone(),
            approval_expires_at_ms: self.authorization.approval_expires_at_ms,
        }
    }
}

fn load_pending_authorization(
    connection: &Connection,
    receipt_id: &str,
) -> Result<Option<StoredPendingApproval>> {
    let base = connection
        .query_row(
            "SELECT requesting_principal, request_id, intent_hash, policy_bundle_hash, state,
                    approval_expires_at_ms, executing_principal, action, target,
                    authorized_action_json, authorized_action_hash, action_binding_hash,
                    capability, adapter_id, adapter_version, environment, tenant,
                    execution_lease_ms
             FROM tlpx_pending_approvals WHERE receipt_id = ?1",
            [receipt_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, String>(16)?,
                    row.get::<_, i64>(17)?,
                ))
            },
        )
        .optional()
        .map_err(db_error)?;
    let Some(base) = base else {
        return Ok(None);
    };
    let mut route_statement = connection
        .prepare(
            "SELECT route_id FROM tlpx_pending_approval_routes
             WHERE receipt_id = ?1 ORDER BY position",
        )
        .map_err(db_error)?;
    let approval_route = route_statement
        .query_map([receipt_id], |row| row.get::<_, String>(0))
        .map_err(db_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let mut scope_statement = connection
        .prepare(
            "SELECT resource FROM tlpx_pending_approval_scopes
             WHERE receipt_id = ?1 ORDER BY position",
        )
        .map_err(db_error)?;
    let resource_scope = scope_statement
        .query_map([receipt_id], |row| row.get::<_, String>(0))
        .map_err(db_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error)?;
    if approval_route.is_empty() || resource_scope.is_empty() {
        return Err(Error::authority(
            "pending approval is missing route or resource scope",
        ));
    }
    assert_hash_string(&base.2)?;
    assert_hash_string(&base.3)?;
    assert_hash_string(&base.10)?;
    assert_hash_string(&base.11)?;
    Ok(Some(StoredPendingApproval {
        state: ApprovalState::parse(&base.4)?,
        authorization: PendingAuthorization {
            receipt_id: receipt_id.to_string(),
            requesting_principal: base.0,
            request_id: base.1,
            intent_hash: base.2,
            policy_bundle_hash: base.3,
            approval_expires_at_ms: base.5,
            executing_principal: base.6,
            action: base.7,
            target: base.8,
            authorized_action_json: base.9,
            authorized_action_hash: base.10,
            action_binding_hash: base.11,
            capability: base.12,
            resource_scope,
            adapter_id: base.13,
            adapter_version: base.14,
            environment: base.15,
            tenant: base.16,
            execution_lease_ms: base.17,
        },
        approval_route,
    }))
}

fn require_approval_operator(
    operator: &AuthenticatedIdentity,
    approval_route: &[String],
) -> Result<()> {
    if operator.party_type() != evidence::PartyType::Human
        || !operator.has_role(LocalRole::Operator)
        || !approval_route
            .iter()
            .any(|route| operator.permits_approval_route(route))
    {
        return Err(Error::coded(
            "APPROVAL_UNAUTHORIZED",
            "authenticated human operator is not authorized for the approval route",
        ));
    }
    Ok(())
}

fn validate_approval_presentation(presentation: &ApprovalPresentation) -> Result<()> {
    assert_hash_string(&presentation.authorized_action_hash).map_err(|_| {
        Error::coded(
            "APPROVAL_PRESENTATION_MISMATCH",
            "approval presentation hash is invalid",
        )
    })?;
    if presentation.renderer_id.is_empty() || presentation.renderer_version.is_empty() {
        return Err(Error::coded(
            "APPROVAL_PRESENTATION_MISMATCH",
            "approval renderer identity and version are required",
        ));
    }
    Ok(())
}

fn expire_pending_transaction(
    transaction: &Transaction<'_>,
    receipt_id: &str,
    expired_at_ms: i64,
) -> Result<i64> {
    let sequence = next_sequence(transaction)?;
    let updated = transaction
        .execute(
            "UPDATE tlpx_pending_approvals
             SET state = 'APPROVAL_EXPIRED', terminal_sequence = ?1, terminal_at_ms = ?2,
                 terminal_actor = NULL, terminal_actor_type = NULL, terminal_role = NULL,
                 terminal_reason = 'APPROVAL_WINDOW_EXPIRED'
             WHERE receipt_id = ?3 AND state = 'PENDING_APPROVAL'",
            params![sequence, expired_at_ms, receipt_id],
        )
        .map_err(db_error)?;
    if updated != 1 {
        return Err(Error::coded(
            "APPROVAL_TERMINAL",
            "pending approval already has a terminal outcome",
        ));
    }
    Ok(sequence)
}

struct DecisionInput<'a> {
    authenticated_requester: &'a str,
    intent: &'a SubmittedIntent,
    intent_hash: &'a str,
    policy: &'a ConfiguredPolicyBundle,
    policy_bundle_hash: &'a str,
    effect: PolicyEffect,
    evaluated_at_ms: i64,
}

fn commit_decision(
    transaction: Transaction<'_>,
    input: DecisionInput<'_>,
    config: &AuthorityConfig,
) -> Result<EvaluationOutcome> {
    let sequence = next_sequence(&transaction)?;
    let receipt_id = match random_id("rcpt") {
        Ok(receipt_id) => receipt_id,
        Err(error) => {
            let fallback = fallback_receipt_id(sequence);
            return commit_preallocated_error(
                transaction,
                &fallback,
                sequence,
                ErrorInput {
                    authenticated_principal: Some(input.authenticated_requester),
                    request_id: Some(&input.intent.request_id),
                    intent_hash: Some(input.intent_hash),
                    retry_of_receipt_id: input.intent.retry_of_receipt_id.as_deref(),
                    stage: "receipt_allocation",
                    code: "AUTHORITY_INTERNAL_ERROR",
                    message: error.message(),
                    retryability: Retryability::AfterCondition,
                    required_condition: Some("operating-system randomness is available"),
                    policy_bundle_id: Some(&input.policy.manifest.policy_bundle_id),
                    policy_bundle_hash: Some(input.policy_bundle_hash),
                    evaluated_at_ms: input.evaluated_at_ms,
                    occupies_slot: true,
                },
                config,
            );
        }
    };
    let action = if matches!(
        input.effect.decision,
        Decision::Allow | Decision::RequireApproval
    ) {
        Some(
            match input.policy.policy.authorize(
                input.intent,
                &input.effect,
                input.policy_bundle_hash,
            ) {
                Ok(action) => action,
                Err(error) => {
                    return commit_preallocated_error(
                        transaction,
                        &receipt_id,
                        sequence,
                        ErrorInput {
                            authenticated_principal: Some(input.authenticated_requester),
                            request_id: Some(&input.intent.request_id),
                            intent_hash: Some(input.intent_hash),
                            retry_of_receipt_id: input.intent.retry_of_receipt_id.as_deref(),
                            stage: "authorization_construction",
                            code: "AUTHORITY_INTERNAL_ERROR",
                            message: error.message(),
                            retryability: Retryability::Never,
                            required_condition: None,
                            policy_bundle_id: Some(&input.policy.manifest.policy_bundle_id),
                            policy_bundle_hash: Some(input.policy_bundle_hash),
                            evaluated_at_ms: input.evaluated_at_ms,
                            occupies_slot: true,
                        },
                        config,
                    );
                }
            },
        )
    } else {
        None
    };
    let mut authorization = None;
    let mut pending_authorization = None;
    let authorization_id = if input.effect.decision == Decision::Allow {
        let action = action
            .as_ref()
            .ok_or_else(|| Error::authority("ALLOW decision has no authorized action"))?;
        let issued = match issue_authorization(
            config,
            input.intent,
            action,
            input.intent_hash,
            &receipt_id,
            input.evaluated_at_ms,
        ) {
            Ok(issued) => issued,
            Err(error) => {
                return commit_preallocated_error(
                    transaction,
                    &receipt_id,
                    sequence,
                    ErrorInput {
                        authenticated_principal: Some(input.authenticated_requester),
                        request_id: Some(&input.intent.request_id),
                        intent_hash: Some(input.intent_hash),
                        retry_of_receipt_id: input.intent.retry_of_receipt_id.as_deref(),
                        stage: "authorization_issuance",
                        code: "AUTHORITY_INTERNAL_ERROR",
                        message: error.message(),
                        retryability: Retryability::Never,
                        required_condition: None,
                        policy_bundle_id: Some(&input.policy.manifest.policy_bundle_id),
                        policy_bundle_hash: Some(input.policy_bundle_hash),
                        evaluated_at_ms: input.evaluated_at_ms,
                        occupies_slot: true,
                    },
                    config,
                );
            }
        };
        if signing_key_is_durably_revoked(&transaction, config)? {
            return commit_preallocated_error(
                transaction,
                &receipt_id,
                sequence,
                ErrorInput {
                    authenticated_principal: Some(input.authenticated_requester),
                    request_id: Some(&input.intent.request_id),
                    intent_hash: Some(input.intent_hash),
                    retry_of_receipt_id: input.intent.retry_of_receipt_id.as_deref(),
                    stage: "authorization_issuance",
                    code: "AUTHORITY_INTERNAL_ERROR",
                    message: "active authorization signing key is durably revoked",
                    retryability: Retryability::AfterCondition,
                    required_condition: Some("a non-revoked authorization signing key is active"),
                    policy_bundle_id: Some(&input.policy.manifest.policy_bundle_id),
                    policy_bundle_hash: Some(input.policy_bundle_hash),
                    evaluated_at_ms: input.evaluated_at_ms,
                    occupies_slot: true,
                },
                config,
            );
        }
        let id = issued.authorization_id.clone();
        authorization = Some(issued);
        Some(id)
    } else if input.effect.decision == Decision::RequireApproval {
        let action = action.as_ref().ok_or_else(|| {
            Error::authority("REQUIRE_APPROVAL decision has no authorized action")
        })?;
        pending_authorization = Some(pending_authorization_from_action(
            config,
            input.intent,
            action,
            input.intent_hash,
            &receipt_id,
            input.evaluated_at_ms,
        )?);
        None
    } else {
        None
    };

    if transaction
        .execute(
            "INSERT INTO tlpx_evaluations (
               sequence, receipt_id, authenticated_principal, request_id, intent_hash,
               retry_of_receipt_id, outcome_kind, decision, policy_id, reason_code, reason,
               retryability, required_condition, policy_bundle_hash, evaluated_at_ms,
               authorization_id, occupies_slot
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'DECISION', ?7, ?8, ?9, ?10,
                       NULL, NULL, ?11, ?12, ?13, 1)",
            params![
                sequence,
                receipt_id,
                input.authenticated_requester,
                input.intent.request_id,
                input.intent_hash,
                input.intent.retry_of_receipt_id,
                input.effect.decision.as_str(),
                input.effect.policy_id,
                input.effect.reason_code,
                input.effect.reason_code,
                input.policy_bundle_hash,
                input.evaluated_at_ms,
                authorization_id,
            ],
        )
        .is_err()
    {
        let fallback = fallback_receipt_id(sequence);
        return commit_preallocated_error(
            transaction,
            &fallback,
            sequence,
            ErrorInput {
                authenticated_principal: Some(input.authenticated_requester),
                request_id: Some(&input.intent.request_id),
                intent_hash: Some(input.intent_hash),
                retry_of_receipt_id: input.intent.retry_of_receipt_id.as_deref(),
                stage: "decision_persistence",
                code: "AUTHORITY_INTERNAL_ERROR",
                message: "decision could not be persisted",
                retryability: Retryability::Never,
                required_condition: None,
                policy_bundle_id: Some(&input.policy.manifest.policy_bundle_id),
                policy_bundle_hash: Some(input.policy_bundle_hash),
                evaluated_at_ms: input.evaluated_at_ms,
                occupies_slot: true,
            },
            config,
        );
    }
    if input.effect.decision == Decision::RequireApproval {
        let route = input
            .effect
            .approval_route
            .as_deref()
            .ok_or_else(|| Error::authority("validated approval decision has no route"))?;
        let pending = pending_authorization.as_ref().ok_or_else(|| {
            Error::authority("validated approval decision has no pending authorization")
        })?;
        if insert_pending_approval(&transaction, pending, route).is_err() {
            transaction
                .execute(
                    "DELETE FROM tlpx_pending_approvals WHERE receipt_id = ?1",
                    [&receipt_id],
                )
                .map_err(db_error)?;
            return replace_decision_with_error(
                transaction,
                &receipt_id,
                sequence,
                "",
                ErrorInput {
                    authenticated_principal: Some(input.authenticated_requester),
                    request_id: Some(&input.intent.request_id),
                    intent_hash: Some(input.intent_hash),
                    retry_of_receipt_id: input.intent.retry_of_receipt_id.as_deref(),
                    stage: "approval_persistence",
                    code: "AUTHORITY_INTERNAL_ERROR",
                    message: "pending approval could not be persisted",
                    retryability: Retryability::Never,
                    required_condition: None,
                    policy_bundle_id: Some(&input.policy.manifest.policy_bundle_id),
                    policy_bundle_hash: Some(input.policy_bundle_hash),
                    evaluated_at_ms: input.evaluated_at_ms,
                    occupies_slot: true,
                },
                config,
            );
        }
    }
    if let Some(ref issued) = authorization {
        if insert_authorization(&transaction, issued).is_err() {
            return replace_decision_with_error(
                transaction,
                &receipt_id,
                sequence,
                &issued.authorization_id,
                ErrorInput {
                    authenticated_principal: Some(input.authenticated_requester),
                    request_id: Some(&input.intent.request_id),
                    intent_hash: Some(input.intent_hash),
                    retry_of_receipt_id: input.intent.retry_of_receipt_id.as_deref(),
                    stage: "authorization_persistence",
                    code: "AUTHORITY_INTERNAL_ERROR",
                    message: "authorization could not be persisted",
                    retryability: Retryability::Never,
                    required_condition: None,
                    policy_bundle_id: Some(&input.policy.manifest.policy_bundle_id),
                    policy_bundle_hash: Some(input.policy_bundle_hash),
                    evaluated_at_ms: input.evaluated_at_ms,
                    occupies_slot: true,
                },
                config,
            );
        }
    }
    let decision_evidence = evidence::decision_record(DecisionEvidenceInput {
        receipt_id: &receipt_id,
        request_id: &input.intent.request_id,
        retry_of_receipt_id: input.intent.retry_of_receipt_id.as_deref(),
        evaluated_at_ms: input.evaluated_at_ms,
        decision: input.effect.decision,
        reason_code: &input.effect.reason_code,
        policy_id: input.effect.policy_id.as_deref(),
        policy_bundle_id: &input.policy.manifest.policy_bundle_id,
        policy_bundle_version: &input.policy.manifest.policy_bundle_version,
        policy_bundle_hash: input.policy_bundle_hash,
        intent_hash: input.intent_hash,
        authenticated_requester: input.authenticated_requester,
        sequence,
        config: &config.evidence,
    })?;
    evidence::enqueue(
        &transaction,
        &config.evidence,
        sequence,
        0,
        "tlpx.decision",
        &receipt_id,
        &decision_evidence,
    )?;
    if let Some(ref issued) = authorization {
        let authorization_evidence = evidence::authorization_record(issued)?;
        evidence::enqueue(
            &transaction,
            &config.evidence,
            sequence,
            1,
            "tlpx.authorization",
            &issued.authorization_id,
            &authorization_evidence,
        )?;
    }
    transaction.commit().map_err(db_error)?;

    Ok(EvaluationOutcome {
        receipt_id,
        sequence,
        authenticated_requester: input.authenticated_requester.to_string(),
        request_id: input.intent.request_id.clone(),
        retry_of_receipt_id: input.intent.retry_of_receipt_id.clone(),
        decision: input.effect.decision,
        reason_code: input.effect.reason_code,
        policy_id: input.effect.policy_id,
        intent_hash: input.intent_hash.to_string(),
        policy_bundle_hash: input.policy_bundle_hash.to_string(),
        authorization,
    })
}

#[derive(Debug, Clone)]
struct PendingAuthorization {
    receipt_id: String,
    requesting_principal: String,
    request_id: String,
    intent_hash: String,
    policy_bundle_hash: String,
    approval_expires_at_ms: i64,
    executing_principal: String,
    action: String,
    target: String,
    authorized_action_json: String,
    authorized_action_hash: String,
    action_binding_hash: String,
    capability: String,
    resource_scope: Vec<String>,
    adapter_id: String,
    adapter_version: String,
    environment: String,
    tenant: String,
    execution_lease_ms: i64,
}

fn pending_authorization_from_action(
    config: &AuthorityConfig,
    intent: &SubmittedIntent,
    action: &AuthorizedAction,
    intent_hash: &str,
    receipt_id: &str,
    evaluated_at_ms: i64,
) -> Result<PendingAuthorization> {
    assert_hash_string(intent_hash)?;
    let approval_expires_at_ms = evaluated_at_ms
        .checked_add(config.approval_window_ms)
        .ok_or_else(|| Error::authority("approval deadline overflow"))?;
    Ok(PendingAuthorization {
        receipt_id: receipt_id.to_string(),
        requesting_principal: action.requesting_principal.clone(),
        request_id: intent.request_id.clone(),
        intent_hash: intent_hash.to_string(),
        policy_bundle_hash: action.policy_bundle_hash.clone(),
        approval_expires_at_ms,
        executing_principal: action.executing_principal.clone(),
        action: action.action.clone(),
        target: action.target.clone(),
        authorized_action_json: canonicalize(&action.to_value())?.as_str().to_string(),
        authorized_action_hash: action.authorized_action_hash()?,
        action_binding_hash: action.binding_hash()?,
        capability: action.capability.clone(),
        resource_scope: action.resource_scope.clone(),
        adapter_id: action.adapter.id.clone(),
        adapter_version: action.adapter.version.clone(),
        environment: action.environment.clone(),
        tenant: action.tenant.clone(),
        execution_lease_ms: config.execution_lease_ms,
    })
}

fn insert_pending_approval(
    transaction: &Transaction<'_>,
    pending: &PendingAuthorization,
    approval_route: &[String],
) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO tlpx_pending_approvals (
               receipt_id, requesting_principal, request_id, intent_hash, policy_bundle_hash,
               state, approval_expires_at_ms, executing_principal, action, target,
               authorized_action_json, authorized_action_hash, action_binding_hash, capability,
               adapter_id, adapter_version, environment, tenant, execution_lease_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'PENDING_APPROVAL', ?6, ?7, ?8, ?9,
                       ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                pending.receipt_id,
                pending.requesting_principal,
                pending.request_id,
                pending.intent_hash,
                pending.policy_bundle_hash,
                pending.approval_expires_at_ms,
                pending.executing_principal,
                pending.action,
                pending.target,
                pending.authorized_action_json,
                pending.authorized_action_hash,
                pending.action_binding_hash,
                pending.capability,
                pending.adapter_id,
                pending.adapter_version,
                pending.environment,
                pending.tenant,
                pending.execution_lease_ms,
            ],
        )
        .map_err(db_error)?;
    for (position, route_id) in approval_route.iter().enumerate() {
        let position = i64::try_from(position)
            .map_err(|_| Error::authority("approval route position overflow"))?;
        transaction
            .execute(
                "INSERT INTO tlpx_pending_approval_routes (receipt_id, route_id, position)
                 VALUES (?1, ?2, ?3)",
                params![pending.receipt_id, route_id, position],
            )
            .map_err(db_error)?;
    }
    for (position, resource) in pending.resource_scope.iter().enumerate() {
        let position = i64::try_from(position)
            .map_err(|_| Error::authority("pending resource scope position overflow"))?;
        transaction
            .execute(
                "INSERT INTO tlpx_pending_approval_scopes (receipt_id, resource, position)
                 VALUES (?1, ?2, ?3)",
                params![pending.receipt_id, resource, position],
            )
            .map_err(db_error)?;
    }
    Ok(())
}

fn replace_decision_with_error<T>(
    transaction: Transaction<'_>,
    receipt_id: &str,
    sequence: i64,
    authorization_id: &str,
    input: ErrorInput<'_>,
    config: &AuthorityConfig,
) -> Result<T> {
    transaction
        .execute(
            "DELETE FROM tlpx_authorizations
             WHERE authorization_id = ?1 AND receipt_id = ?2",
            params![authorization_id, receipt_id],
        )
        .map_err(db_error)?;
    let updated = transaction
        .execute(
            "UPDATE tlpx_evaluations
             SET outcome_kind = 'EVALUATION_ERROR', decision = NULL, stage = ?1,
                 policy_id = NULL, reason_code = 'AUTHORITY_INTERNAL_ERROR', reason = ?2,
                 retryability = 'NEVER', required_condition = NULL, authorization_id = NULL
             WHERE sequence = ?3 AND receipt_id = ?4 AND outcome_kind = 'DECISION'",
            params![input.stage, input.message, sequence, receipt_id],
        )
        .map_err(db_error)?;
    if updated != 1 {
        return Err(Error::authority(
            "failed authorization could not be converted to durable evaluation error",
        ));
    }
    let record = evidence::evaluation_error_record(ErrorEvidenceInput {
        receipt_id,
        occurred_at_ms: input.evaluated_at_ms,
        stage: input.stage,
        error_code: input.code,
        retryability: input.retryability,
        reason: input.message,
        sequence,
        authenticated_requester: input.authenticated_principal,
        request_id: input.request_id,
        intent_hash: input.intent_hash,
        retry_of_receipt_id: input.retry_of_receipt_id,
        required_condition: input.required_condition,
        policy_bundle_id: input.policy_bundle_id,
    })?;
    evidence::enqueue(
        &transaction,
        &config.evidence,
        sequence,
        0,
        "tlpx.evaluation_error",
        receipt_id,
        &record,
    )?;
    transaction.commit().map_err(db_error)?;
    Err(Error::coded(input.code, input.message).with_evidence(receipt_id, sequence))
}

fn issue_authorization(
    config: &AuthorityConfig,
    intent: &SubmittedIntent,
    action: &AuthorizedAction,
    intent_hash: &str,
    receipt_id: &str,
    issued_at_ms: i64,
) -> Result<IssuedAuthorization> {
    assert_hash_string(intent_hash)?;
    let claim_expires_at_ms = issued_at_ms
        .checked_add(config.claim_window_ms)
        .ok_or_else(|| Error::authority("claim deadline overflow"))?;
    let authorization_id = random_id("authz")?;
    let authorized_action_hash = action.authorized_action_hash()?;
    let idempotency_key = evidence::authorization_idempotency_key(
        &authorization_id,
        &action.executing_principal,
        &authorized_action_hash,
    )?;
    let mut issued = IssuedAuthorization {
        authorization_id,
        receipt_id: receipt_id.to_string(),
        requesting_principal: action.requesting_principal.clone(),
        executing_principal: action.executing_principal.clone(),
        request_id: intent.request_id.clone(),
        action: action.action.clone(),
        target: action.target.clone(),
        intent_hash: intent_hash.to_string(),
        authorized_action_hash,
        action_binding_hash: action.binding_hash()?,
        capability: action.capability.clone(),
        policy_bundle_hash: action.policy_bundle_hash.clone(),
        resource_scope: action.resource_scope.clone(),
        adapter_id: action.adapter.id.clone(),
        adapter_version: action.adapter.version.clone(),
        environment: action.environment.clone(),
        tenant: action.tenant.clone(),
        authorization_nonce: random_id("nonce")?,
        authorization_signing_key_id: String::new(),
        authorization_signature: String::new(),
        idempotency_key,
        issued_at_ms,
        claim_expires_at_ms,
        execution_lease_ms: config.execution_lease_ms,
        state: AuthzState::AuthorizedUnclaimed,
    };
    sign_issued_authorization(config, &mut issued)?;
    Ok(issued)
}

fn issue_pending_authorization(
    config: &AuthorityConfig,
    pending: &PendingAuthorization,
    issued_at_ms: i64,
) -> Result<IssuedAuthorization> {
    let claim_expires_at_ms = issued_at_ms
        .checked_add(config.claim_window_ms)
        .ok_or_else(|| Error::authority("claim deadline overflow"))?;
    let authorization_id = random_id("authz")?;
    let idempotency_key = evidence::authorization_idempotency_key(
        &authorization_id,
        &pending.executing_principal,
        &pending.authorized_action_hash,
    )?;
    let mut issued = IssuedAuthorization {
        authorization_id,
        receipt_id: pending.receipt_id.clone(),
        requesting_principal: pending.requesting_principal.clone(),
        executing_principal: pending.executing_principal.clone(),
        request_id: pending.request_id.clone(),
        action: pending.action.clone(),
        target: pending.target.clone(),
        intent_hash: pending.intent_hash.clone(),
        authorized_action_hash: pending.authorized_action_hash.clone(),
        action_binding_hash: pending.action_binding_hash.clone(),
        capability: pending.capability.clone(),
        policy_bundle_hash: pending.policy_bundle_hash.clone(),
        resource_scope: pending.resource_scope.clone(),
        adapter_id: pending.adapter_id.clone(),
        adapter_version: pending.adapter_version.clone(),
        environment: pending.environment.clone(),
        tenant: pending.tenant.clone(),
        authorization_nonce: random_id("nonce")?,
        authorization_signing_key_id: String::new(),
        authorization_signature: String::new(),
        idempotency_key,
        issued_at_ms,
        claim_expires_at_ms,
        execution_lease_ms: pending.execution_lease_ms,
        state: AuthzState::AuthorizedUnclaimed,
    };
    sign_issued_authorization(config, &mut issued)?;
    Ok(issued)
}

struct AuthorizationProofView<'a> {
    authorization_id: &'a str,
    authorization_nonce: &'a str,
    requesting_principal: &'a str,
    executing_principal: &'a str,
    authorized_action_hash: &'a str,
    action_binding_hash: &'a str,
    capability: &'a str,
    policy_bundle_hash: &'a str,
    adapter_id: &'a str,
    adapter_version: &'a str,
    environment: &'a str,
    tenant: &'a str,
    issued_at_ms: i64,
    claim_expires_at_ms: i64,
    execution_lease_ms: i64,
}

impl<'a> AuthorizationProofView<'a> {
    fn issued(value: &'a IssuedAuthorization) -> Self {
        Self {
            authorization_id: &value.authorization_id,
            authorization_nonce: &value.authorization_nonce,
            requesting_principal: &value.requesting_principal,
            executing_principal: &value.executing_principal,
            authorized_action_hash: &value.authorized_action_hash,
            action_binding_hash: &value.action_binding_hash,
            capability: &value.capability,
            policy_bundle_hash: &value.policy_bundle_hash,
            adapter_id: &value.adapter_id,
            adapter_version: &value.adapter_version,
            environment: &value.environment,
            tenant: &value.tenant,
            issued_at_ms: value.issued_at_ms,
            claim_expires_at_ms: value.claim_expires_at_ms,
            execution_lease_ms: value.execution_lease_ms,
        }
    }

    fn stored(authorization_id: &'a str, value: &'a StoredAuthorization) -> Self {
        Self {
            authorization_id,
            authorization_nonce: &value.authorization_nonce,
            requesting_principal: &value.requesting_principal,
            executing_principal: &value.executing_principal,
            authorized_action_hash: &value.authorized_action_hash,
            action_binding_hash: &value.action_binding_hash,
            capability: &value.capability,
            policy_bundle_hash: &value.policy_bundle_hash,
            adapter_id: &value.adapter_id,
            adapter_version: &value.adapter_version,
            environment: &value.environment,
            tenant: &value.tenant,
            issued_at_ms: value.issued_at_ms,
            claim_expires_at_ms: value.claim_expires_at_ms,
            execution_lease_ms: value.execution_lease_ms,
        }
    }
}

fn sign_issued_authorization(
    config: &AuthorityConfig,
    issued: &mut IssuedAuthorization,
) -> Result<()> {
    let payload = authorization_proof_payload(&AuthorizationProofView::issued(issued));
    let proof = config
        .evidence
        .keys
        .sign(KeyPurpose::AuthorizationSigning, &payload)?;
    if proof.algorithm != "HMAC-SHA256" {
        return Err(Error::authority(
            "authorization proof algorithm is unsupported",
        ));
    }
    issued.authorization_signing_key_id = proof.key_id;
    issued.authorization_signature = proof.proof;
    Ok(())
}

fn verify_stored_authorization_proof(
    config: &AuthorityConfig,
    authorization_id: &str,
    stored: &StoredAuthorization,
) -> Result<()> {
    let payload =
        authorization_proof_payload(&AuthorizationProofView::stored(authorization_id, stored));
    config
        .evidence
        .keys
        .verify(
            &stored.authorization_signing_key_id,
            KeyPurpose::AuthorizationSigning,
            &payload,
            &stored.authorization_signature,
        )
        .map_err(|error| match error.code() {
            "KEY_REVOKED" => Error::claim("AUTHORIZATION_REVOKED"),
            _ => Error::claim("AUTHORIZATION_PROOF_INVALID"),
        })
}

fn authorization_proof_payload(value: &AuthorizationProofView<'_>) -> Vec<u8> {
    let mut payload = b"northstar:authorization-proof:v1\0".to_vec();
    for field in [
        value.authorization_id,
        value.authorization_nonce,
        value.requesting_principal,
        value.executing_principal,
        value.authorized_action_hash,
        value.action_binding_hash,
        value.capability,
        value.policy_bundle_hash,
        value.adapter_id,
        value.adapter_version,
        value.environment,
        value.tenant,
    ] {
        let length = u64::try_from(field.len()).expect("string length fits u64");
        payload.extend_from_slice(&length.to_be_bytes());
        payload.extend_from_slice(field.as_bytes());
    }
    payload.extend_from_slice(&value.issued_at_ms.to_be_bytes());
    payload.extend_from_slice(&value.claim_expires_at_ms.to_be_bytes());
    payload.extend_from_slice(&value.execution_lease_ms.to_be_bytes());
    payload
}

struct ErrorInput<'a> {
    authenticated_principal: Option<&'a str>,
    request_id: Option<&'a str>,
    intent_hash: Option<&'a str>,
    retry_of_receipt_id: Option<&'a str>,
    stage: &'a str,
    code: &'a str,
    message: &'a str,
    retryability: Retryability,
    required_condition: Option<&'a str>,
    policy_bundle_id: Option<&'a str>,
    policy_bundle_hash: Option<&'a str>,
    evaluated_at_ms: i64,
    occupies_slot: bool,
}

fn commit_evaluation_error<T>(
    transaction: Transaction<'_>,
    input: ErrorInput<'_>,
    config: &AuthorityConfig,
) -> Result<T> {
    let sequence = next_sequence(&transaction)?;
    let receipt_id = random_id("rcpt").unwrap_or_else(|_| fallback_receipt_id(sequence));
    commit_preallocated_error(transaction, &receipt_id, sequence, input, config)
}

fn commit_preallocated_error<T>(
    transaction: Transaction<'_>,
    receipt_id: &str,
    sequence: i64,
    input: ErrorInput<'_>,
    config: &AuthorityConfig,
) -> Result<T> {
    transaction
        .execute(
            "INSERT INTO tlpx_evaluations (
               sequence, receipt_id, authenticated_principal, request_id, intent_hash,
               retry_of_receipt_id, outcome_kind, decision, stage, reason_code, reason,
               retryability, required_condition, policy_bundle_hash, evaluated_at_ms,
               authorization_id, occupies_slot
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'EVALUATION_ERROR', NULL, ?7, ?8, ?9,
                       ?10, ?11, ?12, ?13, NULL, ?14)",
            params![
                sequence,
                receipt_id,
                input.authenticated_principal,
                input.request_id,
                input.intent_hash,
                input.retry_of_receipt_id,
                input.stage,
                input.code,
                input.message,
                input.retryability.as_str(),
                input.required_condition,
                input.policy_bundle_hash,
                input.evaluated_at_ms,
                i64::from(input.occupies_slot),
            ],
        )
        .map_err(db_error)?;
    let record = evidence::evaluation_error_record(ErrorEvidenceInput {
        receipt_id,
        occurred_at_ms: input.evaluated_at_ms,
        stage: input.stage,
        error_code: input.code,
        retryability: input.retryability,
        reason: input.message,
        sequence,
        authenticated_requester: input.authenticated_principal,
        request_id: input.request_id,
        intent_hash: input.intent_hash,
        retry_of_receipt_id: input.retry_of_receipt_id,
        required_condition: input.required_condition,
        policy_bundle_id: input.policy_bundle_id,
    })?;
    evidence::enqueue(
        &transaction,
        &config.evidence,
        sequence,
        0,
        "tlpx.evaluation_error",
        receipt_id,
        &record,
    )?;
    transaction.commit().map_err(db_error)?;
    Err(Error::coded(input.code, input.message).with_evidence(receipt_id, sequence))
}

fn next_sequence(transaction: &Transaction<'_>) -> Result<i64> {
    let sequence = transaction
        .query_row(
            "SELECT next_value FROM tlpx_sequence WHERE singleton = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(db_error)?;
    let next = sequence
        .checked_add(1)
        .ok_or_else(|| Error::authority("authority sequence overflow"))?;
    let updated = transaction
        .execute(
            "UPDATE tlpx_sequence SET next_value = ?1 WHERE singleton = 1 AND next_value = ?2",
            params![next, sequence],
        )
        .map_err(db_error)?;
    if updated != 1 {
        return Err(Error::authority("authority sequence allocation failed"));
    }
    Ok(sequence)
}

fn retry_link_is_valid(
    transaction: &Transaction<'_>,
    authenticated_principal: &str,
    receipt_id: &str,
) -> Result<bool> {
    let prior = transaction
        .query_row(
            "SELECT authenticated_principal, outcome_kind, retryability
             FROM tlpx_evaluations WHERE receipt_id = ?1",
            [receipt_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(db_error)?;
    let Some((owner, kind, retryability)) = prior else {
        return Ok(false);
    };
    if owner.as_deref() != Some(authenticated_principal) || kind != "EVALUATION_ERROR" {
        return Ok(false);
    }
    let Some(retryability) = retryability else {
        return Ok(false);
    };
    Ok(Retryability::parse(&retryability)? != Retryability::Never)
}

fn insert_authorization(transaction: &Transaction<'_>, issued: &IssuedAuthorization) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO tlpx_authorizations (
               authorization_id, receipt_id, requesting_principal, executing_principal,
               request_id, action, target, intent_hash, authorized_action_hash, action_binding_hash,
               capability, policy_bundle_hash, adapter_id, adapter_version, environment, tenant,
               authorization_nonce, authorization_signing_key_id, authorization_signature,
               issued_at_ms, claim_expires_at_ms, execution_lease_ms, state
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
            params![
                issued.authorization_id,
                issued.receipt_id,
                issued.requesting_principal,
                issued.executing_principal,
                issued.request_id,
                issued.action,
                issued.target,
                issued.intent_hash,
                issued.authorized_action_hash,
                issued.action_binding_hash,
                issued.capability,
                issued.policy_bundle_hash,
                issued.adapter_id,
                issued.adapter_version,
                issued.environment,
                issued.tenant,
                issued.authorization_nonce,
                issued.authorization_signing_key_id,
                issued.authorization_signature,
                issued.issued_at_ms,
                issued.claim_expires_at_ms,
                issued.execution_lease_ms,
                issued.state.as_str(),
            ],
        )
        .map_err(db_error)?;
    for (position, scope) in issued.resource_scope.iter().enumerate() {
        let position = i64::try_from(position)
            .map_err(|_| Error::authority("resource scope position overflow"))?;
        transaction
            .execute(
                "INSERT INTO tlpx_authorization_scopes (authorization_id, resource, position)
                 VALUES (?1, ?2, ?3)",
                params![issued.authorization_id, scope, position],
            )
            .map_err(db_error)?;
    }
    Ok(())
}

struct StoredEvaluation {
    receipt_id: String,
    sequence: i64,
    authenticated_principal: String,
    request_id: String,
    intent_hash: Option<String>,
    retry_of_receipt_id: Option<String>,
    outcome_kind: String,
    decision: Option<String>,
    policy_id: Option<String>,
    reason_code: String,
    reason: String,
    policy_bundle_hash: Option<String>,
    authorization_id: Option<String>,
}

impl StoredEvaluation {
    fn into_result(self, transaction: &Transaction<'_>) -> Result<EvaluationOutcome> {
        if self.outcome_kind == "EVALUATION_ERROR" {
            return Err(Error::coded(self.reason_code, self.reason)
                .with_evidence(self.receipt_id, self.sequence));
        }
        let decision = self
            .decision
            .as_deref()
            .ok_or_else(|| Error::authority("stored decision has no decision value"))?;
        let intent_hash = self
            .intent_hash
            .ok_or_else(|| Error::authority("stored decision has no intent hash"))?;
        let policy_bundle_hash = self
            .policy_bundle_hash
            .ok_or_else(|| Error::authority("stored decision has no policy bundle hash"))?;
        let authorization = self
            .authorization_id
            .as_deref()
            .map(|id| load_issued(transaction, id))
            .transpose()?
            .flatten();
        Ok(EvaluationOutcome {
            receipt_id: self.receipt_id,
            sequence: self.sequence,
            authenticated_requester: self.authenticated_principal,
            request_id: self.request_id,
            retry_of_receipt_id: self.retry_of_receipt_id,
            decision: parse_decision(decision)?,
            reason_code: self.reason_code,
            policy_id: self.policy_id,
            intent_hash,
            policy_bundle_hash,
            authorization,
        })
    }
}

fn load_idempotent(
    transaction: &Transaction<'_>,
    authenticated_principal: &str,
    request_id: &str,
) -> Result<Option<StoredEvaluation>> {
    transaction
        .query_row(
            "SELECT receipt_id, sequence, authenticated_principal, request_id, intent_hash,
                    retry_of_receipt_id, outcome_kind, decision, policy_id, reason_code, reason,
                    policy_bundle_hash, authorization_id
             FROM tlpx_evaluations
             WHERE authenticated_principal = ?1 AND request_id = ?2 AND occupies_slot = 1",
            params![authenticated_principal, request_id],
            |row| {
                Ok(StoredEvaluation {
                    receipt_id: row.get(0)?,
                    sequence: row.get(1)?,
                    authenticated_principal: row.get(2)?,
                    request_id: row.get(3)?,
                    intent_hash: row.get(4)?,
                    retry_of_receipt_id: row.get(5)?,
                    outcome_kind: row.get(6)?,
                    decision: row.get(7)?,
                    policy_id: row.get(8)?,
                    reason_code: row.get(9)?,
                    reason: row.get(10)?,
                    policy_bundle_hash: row.get(11)?,
                    authorization_id: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(db_error)
}

fn load_issued(
    transaction: &Transaction<'_>,
    authorization_id: &str,
) -> Result<Option<IssuedAuthorization>> {
    let row = load_authorization_row(transaction, authorization_id)?;
    row.map(|stored| {
        let idempotency_key = evidence::authorization_idempotency_key(
            authorization_id,
            &stored.executing_principal,
            &stored.authorized_action_hash,
        )?;
        Ok(IssuedAuthorization {
            authorization_id: authorization_id.to_string(),
            receipt_id: stored.receipt_id,
            requesting_principal: stored.requesting_principal,
            executing_principal: stored.executing_principal,
            request_id: stored.request_id,
            action: stored.action,
            target: stored.target,
            intent_hash: stored.intent_hash,
            authorized_action_hash: stored.authorized_action_hash,
            action_binding_hash: stored.action_binding_hash,
            capability: stored.capability,
            policy_bundle_hash: stored.policy_bundle_hash,
            resource_scope: stored.resource_scope,
            adapter_id: stored.adapter_id,
            adapter_version: stored.adapter_version,
            environment: stored.environment,
            tenant: stored.tenant,
            authorization_nonce: stored.authorization_nonce,
            authorization_signing_key_id: stored.authorization_signing_key_id,
            authorization_signature: stored.authorization_signature,
            idempotency_key,
            issued_at_ms: stored.issued_at_ms,
            claim_expires_at_ms: stored.claim_expires_at_ms,
            execution_lease_ms: stored.execution_lease_ms,
            state: stored.state,
        })
    })
    .transpose()
}

struct StoredAuthorization {
    receipt_id: String,
    requesting_principal: String,
    executing_principal: String,
    request_id: String,
    action: String,
    target: String,
    intent_hash: String,
    authorized_action_hash: String,
    action_binding_hash: String,
    capability: String,
    policy_bundle_hash: String,
    resource_scope: Vec<String>,
    adapter_id: String,
    adapter_version: String,
    environment: String,
    tenant: String,
    authorization_nonce: String,
    authorization_signing_key_id: String,
    authorization_signature: String,
    issued_at_ms: i64,
    claim_expires_at_ms: i64,
    execution_lease_ms: i64,
    state: AuthzState,
    revoked_at_ms: Option<i64>,
}

fn load_authorization_row(
    transaction: &Transaction<'_>,
    authorization_id: &str,
) -> Result<Option<StoredAuthorization>> {
    let base = transaction
        .query_row(
            "SELECT receipt_id, requesting_principal, executing_principal, request_id, action,
                    target, intent_hash, authorized_action_hash, action_binding_hash, capability,
                    policy_bundle_hash, adapter_id, adapter_version, environment, tenant,
                    authorization_nonce, authorization_signing_key_id, authorization_signature,
                    issued_at_ms, claim_expires_at_ms, execution_lease_ms, state, revoked_at_ms
             FROM tlpx_authorizations WHERE authorization_id = ?1",
            [authorization_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, String>(16)?,
                    row.get::<_, String>(17)?,
                    row.get::<_, i64>(18)?,
                    row.get::<_, i64>(19)?,
                    row.get::<_, i64>(20)?,
                    row.get::<_, String>(21)?,
                    row.get::<_, Option<i64>>(22)?,
                ))
            },
        )
        .optional()
        .map_err(db_error)?;
    let Some(base) = base else {
        return Ok(None);
    };
    let mut statement = transaction
        .prepare(
            "SELECT resource FROM tlpx_authorization_scopes
             WHERE authorization_id = ?1 ORDER BY position",
        )
        .map_err(db_error)?;
    let resource_scope = statement
        .query_map([authorization_id], |row| row.get::<_, String>(0))
        .map_err(db_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(Some(StoredAuthorization {
        receipt_id: base.0,
        requesting_principal: base.1,
        executing_principal: base.2,
        request_id: base.3,
        action: base.4,
        target: base.5,
        intent_hash: base.6,
        authorized_action_hash: base.7,
        action_binding_hash: base.8,
        capability: base.9,
        policy_bundle_hash: base.10,
        resource_scope,
        adapter_id: base.11,
        adapter_version: base.12,
        environment: base.13,
        tenant: base.14,
        authorization_nonce: base.15,
        authorization_signing_key_id: base.16,
        authorization_signature: base.17,
        issued_at_ms: base.18,
        claim_expires_at_ms: base.19,
        execution_lease_ms: base.20,
        state: AuthzState::parse(&base.21)?,
        revoked_at_ms: base.22,
    }))
}

fn authorization_has_active_revocation(
    transaction: &Transaction<'_>,
    authorization_id: &str,
    stored: &StoredAuthorization,
) -> Result<bool> {
    transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM tlpx_revocations
               WHERE (scope_type = 'AUTHORIZATION' AND scope_id = ?1)
                  OR (scope_type = 'PRINCIPAL' AND scope_id IN (?2, ?3))
                  OR (scope_type = 'POLICY_BUNDLE' AND scope_id = ?4)
                  OR (scope_type = 'TENANT' AND scope_id = ?5)
                  OR (scope_type = 'ENVIRONMENT' AND scope_id = ?6)
                  OR (scope_type = 'CAPABILITY' AND scope_id = ?7)
                  OR (scope_type = 'SIGNING_KEY' AND scope_id = ?8)
             )",
            params![
                authorization_id,
                stored.requesting_principal,
                stored.executing_principal,
                stored.policy_bundle_hash,
                stored.tenant,
                stored.environment,
                stored.capability,
                stored.authorization_signing_key_id,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(db_error)
}

fn signing_key_is_durably_revoked(
    transaction: &Transaction<'_>,
    config: &AuthorityConfig,
) -> Result<bool> {
    let key_id = config
        .evidence
        .keys
        .active_key_id(KeyPurpose::AuthorizationSigning)
        .ok_or_else(|| Error::authority("active authorization signing key is unavailable"))?;
    transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM tlpx_revocations
               WHERE scope_type = 'SIGNING_KEY' AND scope_id = ?1
             )",
            [key_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(db_error)
}

struct ClaimExecutionContext {
    claim_id: String,
    authorization_id: String,
    receipt_id: String,
    requesting_principal: String,
    executing_principal: String,
    intent_hash: String,
    authorized_action_hash: String,
    executed_action_hash: String,
    target: String,
    action: String,
    capability: String,
    policy_bundle_hash: String,
    adapter_id: String,
    adapter_version: String,
    claimed_at_ms: i64,
    lease_expires_at_ms: i64,
}

fn load_claim_execution_context(
    transaction: &Transaction<'_>,
    claim_id: &str,
) -> Result<Option<ClaimExecutionContext>> {
    transaction
        .query_row(
            "SELECT c.claim_id, c.authorization_id, c.receipt_id,
                    a.requesting_principal, c.executing_principal, a.intent_hash,
                    c.authorized_action_hash, c.executed_action_hash, a.target,
                    a.action, a.capability, a.policy_bundle_hash, c.adapter_id, c.adapter_version,
                    c.claimed_at_ms, c.lease_expires_at_ms
             FROM tlpx_claims c
             JOIN tlpx_authorizations a ON a.authorization_id = c.authorization_id
             WHERE c.claim_id = ?1",
            [claim_id],
            |row| {
                Ok(ClaimExecutionContext {
                    claim_id: row.get(0)?,
                    authorization_id: row.get(1)?,
                    receipt_id: row.get(2)?,
                    requesting_principal: row.get(3)?,
                    executing_principal: row.get(4)?,
                    intent_hash: row.get(5)?,
                    authorized_action_hash: row.get(6)?,
                    executed_action_hash: row.get(7)?,
                    target: row.get(8)?,
                    action: row.get(9)?,
                    capability: row.get(10)?,
                    policy_bundle_hash: row.get(11)?,
                    adapter_id: row.get(12)?,
                    adapter_version: row.get(13)?,
                    claimed_at_ms: row.get(14)?,
                    lease_expires_at_ms: row.get(15)?,
                })
            },
        )
        .optional()
        .map_err(db_error)
}

struct StoredExecution {
    execution_id: String,
    claim_id: String,
    authorization_id: String,
    receipt_id: String,
    idempotency_key: String,
    requesting_principal: String,
    executing_principal: String,
    intent_hash: String,
    authorized_action_hash: String,
    executed_action_hash: String,
    target: String,
    policy_bundle_id: String,
    policy_bundle_version: String,
    policy_bundle_hash: String,
    adapter_id: String,
    adapter_version: String,
    adapter_principal: Option<String>,
    adapter_binary_hash: Option<String>,
    started_at_ms: i64,
    lease_expires_at_ms: i64,
    state: ExecutionState,
    outcome_unknown_at_ms: Option<i64>,
    reconciliation_required_at_ms: Option<i64>,
    terminal_sequence: Option<i64>,
    ended_at_ms: Option<i64>,
    result_summary: Option<String>,
    result_hash: Option<String>,
    external_evidence_reference: Option<String>,
    cancellation_outcome: Option<CancellationOutcome>,
}

impl StoredExecution {
    fn lease(&self) -> ExecutionLease {
        ExecutionLease {
            execution_id: self.execution_id.clone(),
            claim_id: self.claim_id.clone(),
            authorization_id: self.authorization_id.clone(),
            idempotency_key: self.idempotency_key.clone(),
            executing_principal: self.executing_principal.clone(),
            started_at_ms: self.started_at_ms,
            lease_expires_at_ms: self.lease_expires_at_ms,
            state: self.state,
        }
    }

    fn receipt(&self) -> Result<ExecutionReceipt> {
        if !self.state.is_terminal() {
            return Err(Error::authority(
                "nonterminal execution does not have a terminal receipt",
            ));
        }
        Ok(ExecutionReceipt {
            execution_id: self.execution_id.clone(),
            claim_id: self.claim_id.clone(),
            authorization_id: self.authorization_id.clone(),
            receipt_id: self.receipt_id.clone(),
            requesting_principal: self.requesting_principal.clone(),
            executing_principal: self.executing_principal.clone(),
            intent_hash: self.intent_hash.clone(),
            authorized_action_hash: self.authorized_action_hash.clone(),
            executed_action_hash: self.executed_action_hash.clone(),
            target: self.target.clone(),
            policy_bundle_id: self.policy_bundle_id.clone(),
            policy_bundle_version: self.policy_bundle_version.clone(),
            policy_bundle_hash: self.policy_bundle_hash.clone(),
            adapter_id: self.adapter_id.clone(),
            adapter_version: self.adapter_version.clone(),
            adapter_principal: self.adapter_principal.clone(),
            adapter_binary_hash: self.adapter_binary_hash.clone(),
            sequence: self
                .terminal_sequence
                .ok_or_else(|| Error::authority("terminal execution sequence is missing"))?,
            started_at_ms: self.started_at_ms,
            ended_at_ms: self
                .ended_at_ms
                .ok_or_else(|| Error::authority("terminal execution timestamp is missing"))?,
            state: self.state,
            result: ExecutionResultEvidence {
                result_summary: self.result_summary.clone(),
                result_hash: self.result_hash.clone(),
                external_evidence_reference: self.external_evidence_reference.clone(),
            },
            cancellation_outcome: self.cancellation_outcome,
        })
    }
}

fn load_execution_by_id(
    transaction: &Transaction<'_>,
    execution_id: &str,
) -> Result<Option<StoredExecution>> {
    load_execution(transaction, "execution_id", execution_id)
}

fn load_execution_by_claim(
    transaction: &Transaction<'_>,
    claim_id: &str,
) -> Result<Option<StoredExecution>> {
    load_execution(transaction, "claim_id", claim_id)
}

fn load_execution(
    transaction: &Transaction<'_>,
    column: &str,
    value: &str,
) -> Result<Option<StoredExecution>> {
    let sql = format!(
        "SELECT execution_id, claim_id, authorization_id, receipt_id, idempotency_key,
                requesting_principal, executing_principal, intent_hash,
                authorized_action_hash, executed_action_hash, target,
                policy_bundle_id, policy_bundle_version, policy_bundle_hash,
                adapter_id, adapter_version, adapter_principal, adapter_binary_hash,
                started_at_ms, lease_expires_at_ms, state,
                outcome_unknown_at_ms, reconciliation_required_at_ms,
                terminal_sequence, ended_at_ms, result_summary, result_hash,
                external_evidence_reference, cancellation_outcome
         FROM tlpx_executions WHERE {column} = ?1"
    );
    transaction
        .query_row(&sql, [value], |row| {
            let state = row.get::<_, String>(20)?;
            let cancellation = row.get::<_, Option<String>>(28)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, String>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, Option<String>>(17)?,
                row.get::<_, i64>(18)?,
                row.get::<_, i64>(19)?,
                state,
                row.get::<_, Option<i64>>(21)?,
                row.get::<_, Option<i64>>(22)?,
                row.get::<_, Option<i64>>(23)?,
                row.get::<_, Option<i64>>(24)?,
                row.get::<_, Option<String>>(25)?,
                row.get::<_, Option<String>>(26)?,
                row.get::<_, Option<String>>(27)?,
                cancellation,
            ))
        })
        .optional()
        .map_err(db_error)?
        .map(|row| {
            Ok(StoredExecution {
                execution_id: row.0,
                claim_id: row.1,
                authorization_id: row.2,
                receipt_id: row.3,
                idempotency_key: row.4,
                requesting_principal: row.5,
                executing_principal: row.6,
                intent_hash: row.7,
                authorized_action_hash: row.8,
                executed_action_hash: row.9,
                target: row.10,
                policy_bundle_id: row.11,
                policy_bundle_version: row.12,
                policy_bundle_hash: row.13,
                adapter_id: row.14,
                adapter_version: row.15,
                adapter_principal: row.16,
                adapter_binary_hash: row.17,
                started_at_ms: row.18,
                lease_expires_at_ms: row.19,
                state: ExecutionState::parse(&row.20)?,
                outcome_unknown_at_ms: row.21,
                reconciliation_required_at_ms: row.22,
                terminal_sequence: row.23,
                ended_at_ms: row.24,
                result_summary: row.25,
                result_hash: row.26,
                external_evidence_reference: row.27,
                cancellation_outcome: row
                    .28
                    .as_deref()
                    .map(CancellationOutcome::parse)
                    .transpose()?,
            })
        })
        .transpose()
}

fn policy_identity_for_hash(
    config: &AuthorityConfig,
    policy_bundle_hash: &str,
) -> Result<(String, String)> {
    for configured in &config.policy.bundles {
        if configured.manifest.manifest_hash()? == policy_bundle_hash {
            return Ok((
                configured.manifest.policy_bundle_id.clone(),
                configured.manifest.policy_bundle_version.clone(),
            ));
        }
    }
    Err(Error::authority(
        "issuing policy identity is unavailable for execution receipt",
    ))
}

fn validate_terminal_execution_input(
    terminal_state: ExecutionState,
    result: &ExecutionResultEvidence,
    cancellation_outcome: Option<CancellationOutcome>,
) -> Result<()> {
    if !terminal_state.is_terminal() {
        return Err(Error::coded(
            "EXECUTION_STATE_INVALID",
            "execution receipt state must be terminal",
        ));
    }
    result.validate()?;
    let terminal_cancellation = matches!(
        cancellation_outcome,
        Some(
            CancellationOutcome::CancelledBeforeSideEffect
                | CancellationOutcome::CancelledDuringExecution
        )
    );
    if (terminal_state == ExecutionState::Cancelled) != terminal_cancellation {
        return Err(Error::coded(
            "EXECUTION_RESULT_INVALID",
            "CANCELLED requires an observed terminal cancellation point",
        ));
    }
    Ok(())
}

fn finalize_execution(
    transaction: &Transaction<'_>,
    config: &AuthorityConfig,
    stored: StoredExecution,
    terminal_state: ExecutionState,
    result: ExecutionResultEvidence,
    cancellation_outcome: Option<CancellationOutcome>,
    ended_at_ms: i64,
) -> Result<ExecutionReceipt> {
    if ended_at_ms < stored.started_at_ms {
        return Err(Error::coded(
            "EXECUTION_TIME_INVALID",
            "execution end cannot precede start",
        ));
    }
    let sequence = next_sequence(transaction)?;
    let updated = transaction
        .execute(
            "UPDATE tlpx_executions
             SET state = ?1, terminal_sequence = ?2, ended_at_ms = ?3,
                 result_summary = ?4, result_hash = ?5,
                 external_evidence_reference = ?6, cancellation_outcome = ?7
             WHERE execution_id = ?8 AND state = ?9 AND terminal_sequence IS NULL",
            params![
                terminal_state.as_str(),
                sequence,
                ended_at_ms,
                result.result_summary,
                result.result_hash,
                result.external_evidence_reference,
                cancellation_outcome.map(CancellationOutcome::as_str),
                stored.execution_id,
                stored.state.as_str(),
            ],
        )
        .map_err(db_error)?;
    if updated != 1 {
        return Err(Error::coded(
            "EXECUTION_TERMINAL",
            "another terminal execution outcome won",
        ));
    }
    let receipt = load_execution_by_id(transaction, &stored.execution_id)?
        .ok_or_else(|| Error::authority("finalized execution is missing"))?
        .receipt()?;
    let record = evidence::execution_record(&receipt)?;
    evidence::enqueue(
        transaction,
        &config.evidence,
        receipt.sequence,
        0,
        "tlpx.execution",
        &receipt.execution_id,
        &record,
    )?;
    Ok(receipt)
}

fn parse_decision(value: &str) -> Result<Decision> {
    match value {
        "ALLOW" => Ok(Decision::Allow),
        "REQUIRE_APPROVAL" => Ok(Decision::RequireApproval),
        "DENY" => Ok(Decision::Deny),
        _ => Err(Error::authority(format!("unknown decision {value}"))),
    }
}

fn nonempty(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

fn fallback_receipt_id(sequence: i64) -> String {
    format!("rcpt_local_{sequence:016x}")
}

fn random_id(prefix: &str) -> Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| Error::authority(format!("CSPRNG unavailable: {error}")))?;
    let mut value = String::with_capacity(prefix.len() + 1 + bytes.len() * 2);
    value.push_str(prefix);
    value.push('_');
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut value, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(value)
}

fn observe_trusted_time(transaction: &Transaction<'_>, observed_at_ms: i64) -> Result<()> {
    if observed_at_ms < 0 {
        return Err(Error::coded(
            "TRUSTED_TIME_INVALID",
            "trusted time must not be negative",
        ));
    }
    transaction
        .execute(
            "INSERT OR IGNORE INTO tlpx_trusted_time (singleton, last_observed_ms)
             VALUES (1, ?1)",
            [observed_at_ms],
        )
        .map_err(db_error)?;
    let updated = transaction
        .execute(
            "UPDATE tlpx_trusted_time SET last_observed_ms = ?1
             WHERE singleton = 1 AND last_observed_ms <= ?1",
            [observed_at_ms],
        )
        .map_err(db_error)?;
    if updated != 1 {
        return Err(Error::coded(
            "TRUSTED_TIME_INVALID",
            "trusted authority time moved backward",
        ));
    }
    Ok(())
}

fn now_ms() -> Result<i64> {
    static ANCHOR: OnceLock<Result<(i64, Instant)>> = OnceLock::new();
    let anchor = ANCHOR.get_or_init(|| {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::authority("system clock before Unix epoch"))?;
        let wall_anchor_ms = i64::try_from(duration.as_millis())
            .map_err(|_| Error::authority("system clock overflow"))?;
        Ok((wall_anchor_ms, Instant::now()))
    });
    let (wall_anchor_ms, monotonic_anchor) = anchor.as_ref().map_err(Clone::clone)?;
    let elapsed_ms = i64::try_from(monotonic_anchor.elapsed().as_millis())
        .map_err(|_| Error::authority("monotonic clock overflow"))?;
    wall_anchor_ms
        .checked_add(elapsed_ms)
        .ok_or_else(|| Error::authority("trusted clock overflow"))
}

fn db_error(error: rusqlite::Error) -> Error {
    Error::authority(format!("database: {error}"))
}

fn verify_existing_schema_compatibility(connection: &Connection) -> Result<()> {
    for (table, required_columns) in [
        ("tlpx_trusted_time", &["last_observed_ms"] as &[&str]),
        ("tlpx_evaluations", &["policy_id"] as &[&str]),
        (
            "tlpx_pending_approvals",
            &[
                "request_id",
                "intent_hash",
                "approval_expires_at_ms",
                "authorized_action_json",
                "authorized_action_hash",
                "action_binding_hash",
                "terminal_at_ms",
            ] as &[&str],
        ),
        (
            "tlpx_operator_actions",
            &["authorized_action_hash", "renderer_id", "renderer_version"] as &[&str],
        ),
        (
            "tlpx_authorizations",
            &[
                "target",
                "authorization_signing_key_id",
                "authorization_signature",
            ] as &[&str],
        ),
        ("tlpx_claims", &["adapter_id", "adapter_version"] as &[&str]),
        (
            "tlpx_revocations",
            &[
                "sequence",
                "scope_type",
                "scope_id",
                "revoking_principal",
                "reason",
                "revoked_at_ms",
            ] as &[&str],
        ),
        (
            "tlpx_executions",
            &[
                "idempotency_key",
                "adapter_principal",
                "adapter_binary_hash",
                "outcome_unknown_at_ms",
                "reconciliation_required_at_ms",
                "terminal_sequence",
                "external_evidence_reference",
                "cancellation_outcome",
            ] as &[&str],
        ),
        (
            "tlpx_evidence_outbox",
            &[
                "authority_sequence",
                "ordinal",
                "record_type",
                "source_id",
                "record_json",
                "record_hash",
                "previous_chain_hash",
                "chain_hash",
                "seal_algorithm",
                "seal_key_id",
                "seal",
                "exported_at_ms",
            ] as &[&str],
        ),
    ] {
        let exists = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get::<_, bool>(0),
            )
            .map_err(db_error)?;
        if !exists {
            continue;
        }

        let mut statement = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(db_error)?;
        let columns = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, bool>(3)?))
            })
            .map_err(db_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error)?;
        if let Some(missing) = required_columns
            .iter()
            .find(|column| !columns.iter().any(|(existing, _)| existing == **column))
        {
            return Err(Error::authority(format!(
                "incompatible pre-release authority database: {table}.{missing} is missing; use a fresh database"
            )));
        }
        if table == "tlpx_evaluations"
            && columns
                .iter()
                .any(|(name, not_null)| name == "policy_bundle_hash" && *not_null)
        {
            return Err(Error::authority(
                "incompatible pre-release authority database: tlpx_evaluations.policy_bundle_hash must permit unavailable-policy errors; use a fresh database",
            ));
        }
        if table == "tlpx_evidence_outbox" {
            let table_sql = connection
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get::<_, String>(0),
                )
                .map_err(db_error)?;
            if !table_sql.contains("'tlpx.operator_action'")
                || !table_sql.contains("'tlpx.execution'")
            {
                return Err(Error::authority(
                    "incompatible pre-release authority database: evidence outbox does not permit all current authority records; use a fresh database",
                ));
            }
        }
        if table == "tlpx_pending_approvals" {
            let table_sql = connection
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get::<_, String>(0),
                )
                .map_err(db_error)?;
            if !table_sql.contains("'APPROVAL_EXPIRED'") {
                return Err(Error::authority(
                    "incompatible pre-release authority database: pending approvals do not support approval expiry; use a fresh database",
                ));
            }
        }
    }
    Ok(())
}
