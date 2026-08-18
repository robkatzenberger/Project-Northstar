//! Durable local TL-PX authority MVP.
//!
//! A single SQLite transaction owns each evaluation outcome. Authenticated
//! idempotency slots are immutable, and one authority-wide sequence orders
//! decisions, evaluation errors, and successful claims. The authenticated
//! principal strings still come from a trusted embedding boundary.

use crate::error::{Error, Result};
use crate::evidence::{
    self, DecisionEvidenceInput, ErrorEvidenceInput, EvidenceConfig, EvidenceReconciliation,
    SealedEvidence,
};
use crate::hash::assert_hash_string;
use crate::policy::{CapabilityRegistry, Decision, PolicyEffect, Switchboard};
use crate::policy_manifest::{ConfiguredPolicyBundle, PolicyCatalog};
use crate::types::{ActionBinding, AuthorizedAction, ExecutedAction, SubmittedIntent};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthzState {
    AuthorizedUnclaimed,
    Claimed,
    Expired,
    Revoked,
}

impl AuthzState {
    pub(crate) fn as_str(self) -> &'static str {
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
    pub evidence: EvidenceConfig,
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
        if self.claim_window_ms <= 0 || self.execution_lease_ms <= 0 {
            return Err(Error::policy_compile(
                "claim and execution windows must be positive",
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

pub struct Authority {
    db: Mutex<Connection>,
    config: AuthorityConfig,
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
                 CREATE TABLE IF NOT EXISTS tlpx_evidence_outbox (
                   outbox_id INTEGER PRIMARY KEY AUTOINCREMENT,
                   authority_sequence INTEGER NOT NULL CHECK(authority_sequence >= 1),
                   ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
                   record_type TEXT NOT NULL CHECK(record_type IN (
                     'tlpx.decision', 'tlpx.evaluation_error',
                     'tlpx.authorization', 'tlpx.authorization_claim'
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

    pub fn evaluate_and_issue(
        &self,
        authenticated_requester: &str,
        intent: &SubmittedIntent,
    ) -> Result<EvaluationOutcome> {
        self.evaluate_and_issue_at(authenticated_requester, intent, now_ms()?)
    }

    pub fn evaluate_and_issue_at(
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

    pub fn claim(
        &self,
        authorization_id: &str,
        authenticated_executor: &str,
        executed: &ExecutedAction,
    ) -> Result<ClaimRecord> {
        self.claim_at(
            authorization_id,
            authenticated_executor,
            executed,
            now_ms()?,
        )
    }

    pub fn claim_at(
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

    pub fn revoke_at(&self, authorization_id: &str, revoked_at_ms: i64) -> Result<()> {
        let connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        let updated = connection
            .execute(
                "UPDATE tlpx_authorizations SET state = 'REVOKED', revoked_at_ms = ?1
                 WHERE authorization_id = ?2 AND state = 'AUTHORIZED_UNCLAIMED'",
                params![revoked_at_ms, authorization_id],
            )
            .map_err(db_error)?;
        if updated != 1 {
            return Err(Error::claim("AUTHORIZATION_TERMINAL"));
        }
        Ok(())
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

    /// Acknowledges one exported row by id and chain hash. Repeating the same
    /// acknowledgement is safe and returns `false`.
    pub fn mark_evidence_exported_at(
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

    /// Verifies canonical payloads, record/chain hashes, HMAC seals, source
    /// rows, and complete authority-to-outbox coverage.
    pub fn reconcile_evidence(&self) -> Result<EvidenceReconciliation> {
        let connection = self
            .db
            .lock()
            .map_err(|_| Error::authority("database lock poisoned"))?;
        evidence::reconcile(&connection, &self.config.evidence)
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
    let mut authorization = None;
    let authorization_id = if input.effect.decision == Decision::Allow {
        let action = match input.policy.policy.authorize(
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
        };
        let issued = match issue_authorization(
            config,
            input.intent,
            &action,
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
        let id = issued.authorization_id.clone();
        authorization = Some(issued);
        Some(id)
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
    Ok(IssuedAuthorization {
        authorization_id: random_id("authz")?,
        receipt_id: receipt_id.to_string(),
        requesting_principal: action.requesting_principal.clone(),
        executing_principal: action.executing_principal.clone(),
        request_id: intent.request_id.clone(),
        action: action.action.clone(),
        target: action.target.clone(),
        intent_hash: intent_hash.to_string(),
        authorized_action_hash: action.authorized_action_hash()?,
        action_binding_hash: action.binding_hash()?,
        capability: action.capability.clone(),
        policy_bundle_hash: action.policy_bundle_hash.clone(),
        resource_scope: action.resource_scope.clone(),
        adapter_id: action.adapter.id.clone(),
        adapter_version: action.adapter.version.clone(),
        environment: action.environment.clone(),
        tenant: action.tenant.clone(),
        authorization_nonce: random_id("nonce")?,
        issued_at_ms,
        claim_expires_at_ms,
        execution_lease_ms: config.execution_lease_ms,
        state: AuthzState::AuthorizedUnclaimed,
    })
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
               authorization_nonce, issued_at_ms, claim_expires_at_ms,
               execution_lease_ms, state
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
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
                    authorization_nonce, issued_at_ms, claim_expires_at_ms,
                    execution_lease_ms, state, revoked_at_ms
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
                    row.get::<_, i64>(16)?,
                    row.get::<_, i64>(17)?,
                    row.get::<_, i64>(18)?,
                    row.get::<_, String>(19)?,
                    row.get::<_, Option<i64>>(20)?,
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
        issued_at_ms: base.16,
        claim_expires_at_ms: base.17,
        execution_lease_ms: base.18,
        state: AuthzState::parse(&base.19)?,
        revoked_at_ms: base.20,
    }))
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

fn now_ms() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::authority("system clock before Unix epoch"))?;
    i64::try_from(duration.as_millis()).map_err(|_| Error::authority("system clock overflow"))
}

fn db_error(error: rusqlite::Error) -> Error {
    Error::authority(format!("database: {error}"))
}

fn verify_existing_schema_compatibility(connection: &Connection) -> Result<()> {
    for (table, required_columns) in [
        ("tlpx_evaluations", &["policy_id"] as &[&str]),
        ("tlpx_authorizations", &["target"] as &[&str]),
        ("tlpx_claims", &["adapter_id", "adapter_version"] as &[&str]),
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
    }
    Ok(())
}
