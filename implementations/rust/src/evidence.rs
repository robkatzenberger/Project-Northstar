//! Schema-shaped TL-PX 0.2 evidence plus a sealed durable local outbox.
//!
//! The TL-PX record JSON remains independent of the local storage envelope.
//! Chain hashes and HMAC seals are outbox columns, not private record fields.

use crate::authority::{
    ApprovalResolution, CancellationRecord, ClaimRecord, ExecutionReceipt, IssuedAuthorization,
    Retryability,
};
use crate::error::{Error, Result};
use crate::jcs::{canonicalize, parse, Canonical, Value};
use crate::keys::{KeyProof, KeyPurpose, KeyRing};
use crate::policy::Decision;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use std::fmt;

const RECORD_HASH_PREFIX: &[u8] = b"northstar:evidence-record:v1\0";
const CHAIN_HASH_PREFIX: &[u8] = b"northstar:evidence-chain:v1\0";
const SEAL_PREFIX: &[u8] = b"northstar:evidence-seal:v1\0";
const EXPORT_ACK_PREFIX: &[u8] = b"northstar:evidence-export-ack:v1\0";
const IDEMPOTENCY_PREFIX: &[u8] = b"northstar:idempotency-key:v1\0";
const GENESIS: &str = "GENESIS";
const EXPORT_FORMAT: &str = "tlpx.local-audit-export";
const EXPORT_FORMAT_VERSION: &str = "1";
pub const MAX_AUDIT_SINK_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_AUDIT_LINE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyType {
    Human,
    Machine,
}

impl PartyType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Machine => "machine",
        }
    }
}

#[derive(Clone)]
pub struct EvidenceConfig {
    pub evaluator_id: String,
    pub router_id: String,
    /// Party type asserted by this trusted embedding for its requesters.
    /// Use separate authority instances when requester populations differ.
    pub requester_type: PartyType,
    pub keys: KeyRing,
    /// Exact upper bound for the canonical sealed outbox export. The local
    /// profile never permits a value above `MAX_AUDIT_SINK_BYTES`.
    pub max_export_bytes: u64,
}

impl fmt::Debug for EvidenceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvidenceConfig")
            .field("evaluator_id", &self.evaluator_id)
            .field("router_id", &self.router_id)
            .field("requester_type", &self.requester_type)
            .field("keys", &self.keys)
            .field("max_export_bytes", &self.max_export_bytes)
            .finish()
    }
}

impl EvidenceConfig {
    pub(crate) fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("evaluator_id", self.evaluator_id.as_str()),
            ("router_id", self.router_id.as_str()),
        ] {
            if value.is_empty() {
                return Err(Error::authority(format!("{name} must be non-empty")));
            }
        }
        if !(1..=MAX_AUDIT_SINK_BYTES).contains(&self.max_export_bytes) {
            return Err(Error::authority(format!(
                "max_export_bytes must be 1..={MAX_AUDIT_SINK_BYTES}"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedEvidence {
    pub outbox_id: i64,
    pub authority_sequence: i64,
    pub ordinal: i64,
    pub record_type: String,
    pub source_id: String,
    pub record_json: String,
    pub record_hash: String,
    pub previous_chain_hash: Option<String>,
    pub chain_hash: String,
    pub seal_algorithm: String,
    pub seal_key_id: String,
    pub seal: String,
    pub exported_at_ms: Option<i64>,
    pub export_ack_key_id: Option<String>,
    pub export_ack: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReconciliation {
    pub total: i64,
    pub pending: i64,
    pub exported: i64,
    pub last_chain_hash: Option<String>,
}

pub(crate) struct DecisionEvidenceInput<'a> {
    pub receipt_id: &'a str,
    pub request_id: &'a str,
    pub retry_of_receipt_id: Option<&'a str>,
    pub evaluated_at_ms: i64,
    pub decision: Decision,
    pub reason_code: &'a str,
    pub policy_id: Option<&'a str>,
    pub policy_bundle_id: &'a str,
    pub policy_bundle_version: &'a str,
    pub policy_bundle_hash: &'a str,
    pub intent_hash: &'a str,
    pub authenticated_requester: &'a str,
    pub sequence: i64,
    pub config: &'a EvidenceConfig,
}

pub(crate) struct ErrorEvidenceInput<'a> {
    pub receipt_id: &'a str,
    pub occurred_at_ms: i64,
    pub stage: &'a str,
    pub error_code: &'a str,
    pub retryability: Retryability,
    pub reason: &'a str,
    pub sequence: i64,
    pub authenticated_requester: Option<&'a str>,
    pub request_id: Option<&'a str>,
    pub intent_hash: Option<&'a str>,
    pub retry_of_receipt_id: Option<&'a str>,
    pub required_condition: Option<&'a str>,
    pub policy_bundle_id: Option<&'a str>,
}

pub(crate) struct CancellationEvidenceInput<'a> {
    pub record: &'a CancellationRecord,
    pub party_type: PartyType,
}

pub(crate) struct ApprovalEvidenceInput<'a> {
    pub record: &'a ApprovalResolution,
}

pub(crate) fn decision_record(input: DecisionEvidenceInput<'_>) -> Result<Canonical> {
    let authorization_state = match input.decision {
        Decision::Allow => "AUTHORIZED_UNCLAIMED",
        Decision::RequireApproval => "PENDING_APPROVAL",
        Decision::Deny => "DENIED",
    };
    let mut fields = vec![
        string("record_type", "tlpx.decision"),
        string("standard", "TL-PX"),
        string("standard_version", "0.2.0"),
        string("control_mode", "ALLOW_ESCALATE_OR_DENY"),
        string("receipt_id", input.receipt_id),
        string("request_id", input.request_id),
        string("evaluated_at", &iso8601_from_ms(input.evaluated_at_ms)?),
        string("decision", input.decision.as_str()),
        string("authorization_state", authorization_state),
        string("reason", input.reason_code),
        string("reason_code", input.reason_code),
        (
            "policy_id".into(),
            input
                .policy_id
                .map_or(Value::Null, |value| Value::String(value.into())),
        ),
        string("policy_bundle_id", input.policy_bundle_id),
        string("policy_bundle_version", input.policy_bundle_version),
        string("policy_bundle_hash", input.policy_bundle_hash),
        string("intent_hash", input.intent_hash),
        (
            "parties".into(),
            Value::Object(vec![
                (
                    "requester".into(),
                    party(
                        input.authenticated_requester,
                        input.config.requester_type.as_str(),
                    ),
                ),
                (
                    "evaluator".into(),
                    party(&input.config.evaluator_id, "machine"),
                ),
                ("router".into(), party(&input.config.router_id, "machine")),
            ]),
        ),
        ("sequence".into(), Value::Int(input.sequence)),
    ];
    if let Some(receipt_id) = input.retry_of_receipt_id {
        fields.push(string("retry_of_receipt_id", receipt_id));
    }
    canonicalize(&Value::Object(fields))
}

pub(crate) fn cancellation_record(input: CancellationEvidenceInput<'_>) -> Result<Canonical> {
    canonicalize(&Value::Object(vec![
        string("record_type", "tlpx.operator_action"),
        string("standard", "TL-PX"),
        string("standard_version", "0.2.0"),
        string("receipt_id", &input.record.receipt_id),
        string("acted_at", &iso8601_from_ms(input.record.cancelled_at_ms)?),
        string("outcome", "CANCEL"),
        (
            "operator".into(),
            party(&input.record.canceller, input.party_type.as_str()),
        ),
        string("policy_bundle_hash", &input.record.policy_bundle_hash),
        ("sequence".into(), Value::Int(input.record.sequence)),
    ]))
}

pub(crate) fn approval_record(input: ApprovalEvidenceInput<'_>) -> Result<Canonical> {
    canonicalize(&Value::Object(vec![
        string("record_type", "tlpx.operator_action"),
        string("standard", "TL-PX"),
        string("standard_version", "0.2.0"),
        string("receipt_id", &input.record.receipt_id),
        string("acted_at", &iso8601_from_ms(input.record.acted_at_ms)?),
        string("outcome", input.record.outcome.as_str()),
        ("operator".into(), party(&input.record.operator, "human")),
        string(
            "authorized_action_hash",
            &input.record.authorized_action_hash,
        ),
        string("policy_bundle_hash", &input.record.policy_bundle_hash),
        (
            "approval_route".into(),
            Value::Array(
                input
                    .record
                    .approval_route
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        ),
        string("renderer_id", &input.record.renderer_id),
        string("renderer_version", &input.record.renderer_version),
        ("sequence".into(), Value::Int(input.record.sequence)),
    ]))
}

pub(crate) fn evaluation_error_record(input: ErrorEvidenceInput<'_>) -> Result<Canonical> {
    let mut fields = vec![
        string("record_type", "tlpx.evaluation_error"),
        string("standard", "TL-PX"),
        string("standard_version", "0.2.0"),
        string("receipt_id", input.receipt_id),
        string("occurred_at", &iso8601_from_ms(input.occurred_at_ms)?),
        string("stage", input.stage),
        string("error_code", input.error_code),
        string("retryability", input.retryability.as_str()),
        string("reason", input.reason),
        ("sequence".into(), Value::Int(input.sequence)),
    ];
    if let (Some(requester), Some(request_id)) = (input.authenticated_requester, input.request_id) {
        fields.push(string("authenticated_requester", requester));
        fields.push(string("request_id", request_id));
    }
    if let Some(intent_hash) = input.intent_hash {
        fields.push(string("intent_hash", intent_hash));
    }
    if let Some(receipt_id) = input.retry_of_receipt_id {
        fields.push(string("retry_of_receipt_id", receipt_id));
    }
    if let Some(condition) = input.required_condition {
        fields.push(string("required_condition", condition));
    }
    if let Some(policy_bundle_id) = input.policy_bundle_id {
        fields.push(string("policy_bundle_id", policy_bundle_id));
    }
    canonicalize(&Value::Object(fields))
}

pub(crate) fn authorization_idempotency_key(
    authorization_id: &str,
    executing_principal: &str,
    authorized_action_hash: &str,
) -> Result<String> {
    Ok(prefixed_sha256(
        IDEMPOTENCY_PREFIX,
        canonicalize(&Value::Object(vec![
            string("authorization_id", authorization_id),
            string("executing_principal", executing_principal),
            string("authorized_action_hash", authorized_action_hash),
        ]))?
        .as_str()
        .as_bytes(),
    ))
}

pub(crate) fn authorization_record(issued: &IssuedAuthorization) -> Result<Canonical> {
    let lease_seconds = issued
        .execution_lease_ms
        .checked_div(1_000)
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| Error::authority("execution lease is not whole positive seconds"))?;
    canonicalize(&Value::Object(vec![
        string("record_type", "tlpx.authorization"),
        string("standard", "TL-PX"),
        string("standard_version", "0.2.0"),
        string("authorization_id", &issued.authorization_id),
        string("receipt_id", &issued.receipt_id),
        string("requesting_principal", &issued.requesting_principal),
        string("executing_principal", &issued.executing_principal),
        string("action", &issued.action),
        string("target", &issued.target),
        string("authorized_action_hash", &issued.authorized_action_hash),
        string("action_binding_hash", &issued.action_binding_hash),
        string("intent_hash", &issued.intent_hash),
        string("environment", &issued.environment),
        string("tenant", &issued.tenant),
        (
            "adapter".into(),
            adapter(&issued.adapter_id, &issued.adapter_version),
        ),
        string("issued_at", &iso8601_from_ms(issued.issued_at_ms)?),
        string(
            "claim_expires_at",
            &iso8601_from_ms(issued.claim_expires_at_ms)?,
        ),
        ("execution_lease_seconds".into(), Value::Int(lease_seconds)),
        string("authorization_nonce", &issued.authorization_nonce),
        string("idempotency_key", &issued.idempotency_key),
        string("state", "AUTHORIZED_UNCLAIMED"),
    ]))
}

pub(crate) fn claim_record(claim: &ClaimRecord) -> Result<Canonical> {
    canonicalize(&Value::Object(vec![
        string("record_type", "tlpx.authorization_claim"),
        string("standard", "TL-PX"),
        string("standard_version", "0.2.0"),
        string("claim_id", &claim.claim_id),
        string("authorization_id", &claim.authorization_id),
        string("receipt_id", &claim.receipt_id),
        string("executing_principal", &claim.executing_principal),
        string("authorized_action_hash", &claim.authorized_action_hash),
        string("action_binding_hash", &claim.action_binding_hash),
        string("executed_action_hash", &claim.executed_action_hash),
        (
            "adapter".into(),
            adapter(&claim.adapter_id, &claim.adapter_version),
        ),
        string("claimed_at", &iso8601_from_ms(claim.claimed_at_ms)?),
        string(
            "lease_expires_at",
            &iso8601_from_ms(claim.lease_expires_at_ms)?,
        ),
        ("sequence".into(), Value::Int(claim.sequence)),
        string("state", "CLAIMED"),
    ]))
}

pub(crate) fn execution_record(receipt: &ExecutionReceipt) -> Result<Canonical> {
    if !receipt.state.is_terminal() {
        return Err(Error::authority(
            "only terminal execution state may produce tlpx.execution evidence",
        ));
    }
    let nullable_string = |value: &Option<String>| {
        value
            .as_ref()
            .map_or(Value::Null, |value| Value::String(value.clone()))
    };
    canonicalize(&Value::Object(vec![
        string("record_type", "tlpx.execution"),
        string("standard", "TL-PX"),
        string("standard_version", "0.2.0"),
        string("execution_id", &receipt.execution_id),
        string("claim_id", &receipt.claim_id),
        string("authorization_id", &receipt.authorization_id),
        string("receipt_id", &receipt.receipt_id),
        string("requesting_principal", &receipt.requesting_principal),
        string("executing_principal", &receipt.executing_principal),
        string("intent_hash", &receipt.intent_hash),
        string("authorized_action_hash", &receipt.authorized_action_hash),
        string("executed_action_hash", &receipt.executed_action_hash),
        string("target", &receipt.target),
        string("policy_bundle_id", &receipt.policy_bundle_id),
        string("policy_bundle_version", &receipt.policy_bundle_version),
        string("policy_bundle_hash", &receipt.policy_bundle_hash),
        (
            "adapter".into(),
            adapter(&receipt.adapter_id, &receipt.adapter_version),
        ),
        (
            "adapter_principal".into(),
            nullable_string(&receipt.adapter_principal),
        ),
        (
            "adapter_binary_hash".into(),
            nullable_string(&receipt.adapter_binary_hash),
        ),
        ("sequence".into(), Value::Int(receipt.sequence)),
        string("started_at", &iso8601_from_ms(receipt.started_at_ms)?),
        string("ended_at", &iso8601_from_ms(receipt.ended_at_ms)?),
        string("state", receipt.state.as_str()),
        (
            "result_summary".into(),
            nullable_string(&receipt.result.result_summary),
        ),
        (
            "result_hash".into(),
            nullable_string(&receipt.result.result_hash),
        ),
        (
            "external_evidence_reference".into(),
            nullable_string(&receipt.result.external_evidence_reference),
        ),
        (
            "cancellation_outcome".into(),
            receipt.cancellation_outcome.map_or(Value::Null, |outcome| {
                Value::String(outcome.as_str().into())
            }),
        ),
    ]))
}

pub(crate) fn enqueue(
    transaction: &Transaction<'_>,
    config: &EvidenceConfig,
    authority_sequence: i64,
    ordinal: i64,
    record_type: &str,
    source_id: &str,
    record: &Canonical,
) -> Result<()> {
    let previous_chain_hash = transaction
        .query_row(
            "SELECT chain_hash FROM tlpx_evidence_outbox ORDER BY outbox_id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(db_error)?;
    let record_hash = prefixed_sha256(RECORD_HASH_PREFIX, record.as_str().as_bytes());
    let chain_hash = compute_chain_hash(
        previous_chain_hash.as_deref(),
        authority_sequence,
        ordinal,
        &record_hash,
    );
    let seal = compute_seal(config, &chain_hash)?;
    if key_is_durably_revoked(transaction, &seal.key_id)? {
        return Err(Error::coded(
            "KEY_REVOKED",
            "revoked audit sealing key cannot create new evidence",
        ));
    }
    transaction
        .execute(
            "INSERT INTO tlpx_evidence_outbox (
               authority_sequence, ordinal, record_type, source_id, record_json,
               record_hash, previous_chain_hash, chain_hash, seal_algorithm,
               seal_key_id, seal, exported_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'HMAC-SHA256', ?9, ?10, NULL)",
            params![
                authority_sequence,
                ordinal,
                record_type,
                source_id,
                record.as_str(),
                record_hash,
                previous_chain_hash,
                chain_hash,
                seal.key_id,
                seal.proof,
            ],
        )
        .map_err(db_error)?;
    verify_export_capacity(transaction, config.max_export_bytes)?;
    Ok(())
}

pub(crate) fn pending(connection: &Connection, limit: usize) -> Result<Vec<SealedEvidence>> {
    if !(1..=1_000).contains(&limit) {
        return Err(Error::authority("evidence outbox limit must be 1..=1000"));
    }
    let limit = i64::try_from(limit).map_err(|_| Error::authority("outbox limit overflow"))?;
    let mut statement = connection
        .prepare(
            "SELECT outbox_id, authority_sequence, ordinal, record_type, source_id,
                    record_json, record_hash, previous_chain_hash, chain_hash,
                    seal_algorithm, seal_key_id, seal, exported_at_ms,
                    export_ack_key_id, export_ack
             FROM tlpx_evidence_outbox
             WHERE exported_at_ms IS NULL
             ORDER BY outbox_id
             LIMIT ?1",
        )
        .map_err(db_error)?;
    let rows = statement
        .query_map([limit], row_to_evidence)
        .map_err(db_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(rows)
}

pub(crate) fn all(connection: &Connection) -> Result<Vec<SealedEvidence>> {
    let mut statement = connection
        .prepare(
            "SELECT outbox_id, authority_sequence, ordinal, record_type, source_id,
                    record_json, record_hash, previous_chain_hash, chain_hash,
                    seal_algorithm, seal_key_id, seal, exported_at_ms,
                    export_ack_key_id, export_ack
             FROM tlpx_evidence_outbox
             ORDER BY outbox_id",
        )
        .map_err(db_error)?;
    let rows = statement
        .query_map([], row_to_evidence)
        .map_err(db_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(rows)
}

pub(crate) fn mark_exported(
    connection: &mut Connection,
    config: &EvidenceConfig,
    outbox_id: i64,
    expected_chain_hash: &str,
    exported_at_ms: i64,
) -> Result<bool> {
    if exported_at_ms < 0 {
        return Err(Error::authority("export timestamp must not be negative"));
    }
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(db_error)?;
    reconcile(&transaction, config)?;
    let acknowledgement =
        compute_export_ack(config, outbox_id, expected_chain_hash, exported_at_ms)?;
    if key_is_durably_revoked(&transaction, &acknowledgement.key_id)? {
        return Err(Error::coded(
            "KEY_REVOKED",
            "revoked audit sealing key cannot acknowledge evidence export",
        ));
    }
    let updated = transaction
        .execute(
            "UPDATE tlpx_evidence_outbox
             SET exported_at_ms = ?1, export_ack_key_id = ?4, export_ack = ?5
             WHERE outbox_id = ?2 AND chain_hash = ?3 AND exported_at_ms IS NULL
               AND outbox_id = (
                 SELECT MIN(outbox_id) FROM tlpx_evidence_outbox WHERE exported_at_ms IS NULL
               )
               AND ?1 >= COALESCE(
                 (SELECT MAX(exported_at_ms) FROM tlpx_evidence_outbox), 0
               )",
            params![
                exported_at_ms,
                outbox_id,
                expected_chain_hash,
                acknowledgement.key_id,
                acknowledgement.proof,
            ],
        )
        .map_err(db_error)?;
    if updated == 1 {
        transaction.commit().map_err(db_error)?;
        return Ok(true);
    }
    let existing = transaction
        .query_row(
            "SELECT chain_hash, exported_at_ms FROM tlpx_evidence_outbox WHERE outbox_id = ?1",
            [outbox_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .optional()
        .map_err(db_error)?;
    transaction.commit().map_err(db_error)?;
    match existing {
        Some((chain_hash, Some(_))) if chain_hash == expected_chain_hash => Ok(false),
        Some((chain_hash, None)) if chain_hash == expected_chain_hash => Err(Error::authority(
            "evidence export acknowledgements must preserve outbox order and time",
        )),
        Some(_) => Err(Error::authority("evidence export acknowledgement mismatch")),
        None => Err(Error::authority("unknown evidence outbox row")),
    }
}

pub(crate) fn reconcile(
    connection: &Connection,
    config: &EvidenceConfig,
) -> Result<EvidenceReconciliation> {
    let mut statement = connection
        .prepare(
            "SELECT outbox_id, authority_sequence, ordinal, record_type, source_id,
                    record_json, record_hash, previous_chain_hash, chain_hash,
                    seal_algorithm, seal_key_id, seal, exported_at_ms,
                    export_ack_key_id, export_ack
             FROM tlpx_evidence_outbox ORDER BY outbox_id",
        )
        .map_err(db_error)?;
    let mut expected_previous: Option<String> = None;
    let mut total = 0_i64;
    let mut export_bytes = 0_u64;
    let mut pending_count = 0_i64;
    let mut exported_count = 0_i64;
    let mut saw_pending = false;
    let mut last_exported_at_ms: Option<i64> = None;
    let mut rows = statement.query_map([], row_to_evidence).map_err(db_error)?;
    for row in &mut rows {
        let row = row.map_err(db_error)?;
        total = total
            .checked_add(1)
            .ok_or_else(|| Error::authority("outbox count overflow"))?;
        if key_is_durably_revoked(connection, &row.seal_key_id)? {
            return Err(Error::coded(
                "KEY_REVOKED",
                "evidence was sealed by a durably revoked key",
            ));
        }
        if row.previous_chain_hash != expected_previous {
            return Err(Error::authority("evidence chain predecessor mismatch"));
        }
        if row.seal_algorithm != "HMAC-SHA256" {
            return Err(Error::authority("evidence seal algorithm mismatch"));
        }
        let parsed = parse(&row.record_json)?;
        let canonical = canonicalize(&parsed)?;
        if canonical.as_str() != row.record_json {
            return Err(Error::authority("evidence record is not canonical JCS"));
        }
        let record_hash = prefixed_sha256(RECORD_HASH_PREFIX, row.record_json.as_bytes());
        if record_hash != row.record_hash {
            return Err(Error::authority("evidence record hash mismatch"));
        }
        let chain_hash = compute_chain_hash(
            row.previous_chain_hash.as_deref(),
            row.authority_sequence,
            row.ordinal,
            &row.record_hash,
        );
        if chain_hash != row.chain_hash {
            return Err(Error::authority("evidence chain hash mismatch"));
        }
        verify_seal(config, &row.seal_key_id, &row.chain_hash, &row.seal)?;
        verify_envelope_binding(&parsed, &row)?;
        verify_source(connection, &row, &parsed)?;
        export_bytes = add_export_row_bytes(export_bytes, &row, config.max_export_bytes)?;
        expected_previous = Some(row.chain_hash.clone());
        if let Some(exported_at_ms) = row.exported_at_ms {
            if saw_pending {
                return Err(Error::authority(
                    "exported evidence rows must form an ordered outbox prefix",
                ));
            }
            if exported_at_ms < 0
                || last_exported_at_ms.is_some_and(|previous| exported_at_ms < previous)
            {
                return Err(Error::authority(
                    "evidence export timestamps must be nonnegative and monotonic",
                ));
            }
            last_exported_at_ms = Some(exported_at_ms);
            let Some(export_ack_key_id) = row.export_ack_key_id.as_deref() else {
                return Err(Error::authority(
                    "evidence export acknowledgement key is missing",
                ));
            };
            let Some(export_ack) = row.export_ack.as_deref() else {
                return Err(Error::authority(
                    "evidence export acknowledgement MAC is missing",
                ));
            };
            if key_is_durably_revoked(connection, export_ack_key_id)? {
                return Err(Error::coded(
                    "KEY_REVOKED",
                    "evidence export acknowledgement uses a durably revoked key",
                ));
            }
            verify_export_ack(
                config,
                export_ack_key_id,
                row.outbox_id,
                &row.chain_hash,
                exported_at_ms,
                export_ack,
            )?;
            exported_count += 1;
        } else {
            if row.export_ack_key_id.is_some() || row.export_ack.is_some() {
                return Err(Error::authority(
                    "pending evidence must not carry an export acknowledgement",
                ));
            }
            saw_pending = true;
            pending_count += 1;
        }
    }
    drop(rows);
    drop(statement);
    verify_coverage(connection)?;
    Ok(EvidenceReconciliation {
        total,
        pending: pending_count,
        exported: exported_count,
        last_chain_hash: expected_previous,
    })
}

fn verify_export_capacity(connection: &Connection, maximum: u64) -> Result<u64> {
    let mut statement = connection
        .prepare(
            "SELECT outbox_id, authority_sequence, ordinal, record_type, source_id,
                    record_json, record_hash, previous_chain_hash, chain_hash,
                    seal_algorithm, seal_key_id, seal, exported_at_ms,
                    export_ack_key_id, export_ack
             FROM tlpx_evidence_outbox ORDER BY outbox_id",
        )
        .map_err(db_error)?;
    let mut rows = statement.query_map([], row_to_evidence).map_err(db_error)?;
    let mut total = 0_u64;
    for row in &mut rows {
        total = add_export_row_bytes(total, &row.map_err(db_error)?, maximum)?;
    }
    Ok(total)
}

fn add_export_row_bytes(current: u64, row: &SealedEvidence, maximum: u64) -> Result<u64> {
    let envelope = export_envelope(row)?;
    if envelope.as_str().len() > MAX_AUDIT_LINE_BYTES {
        return Err(Error::coded(
            "AUDIT_CAPACITY_EXCEEDED",
            "sealed evidence row exceeds the bounded export line size",
        ));
    }
    let row_bytes = u64::try_from(envelope.as_str().len() + 1)
        .map_err(|_| Error::authority("audit export row length overflow"))?;
    let projected = current
        .checked_add(row_bytes)
        .ok_or_else(|| Error::coded("AUDIT_CAPACITY_EXCEEDED", "audit capacity overflow"))?;
    if projected > maximum {
        return Err(Error::coded(
            "AUDIT_CAPACITY_EXCEEDED",
            "sealed evidence capacity is exhausted; authority transition denied before commit",
        ));
    }
    Ok(projected)
}

pub(crate) fn export_envelope(row: &SealedEvidence) -> Result<Canonical> {
    let record = parse(&row.record_json)?;
    canonicalize(&Value::Object(vec![
        string("format", EXPORT_FORMAT),
        string("format_version", EXPORT_FORMAT_VERSION),
        ("outbox_id".into(), Value::Int(row.outbox_id)),
        (
            "authority_sequence".into(),
            Value::Int(row.authority_sequence),
        ),
        ("ordinal".into(), Value::Int(row.ordinal)),
        string("record_type", &row.record_type),
        string("source_id", &row.source_id),
        ("record".into(), record),
        string("record_hash", &row.record_hash),
        (
            "previous_chain_hash".into(),
            row.previous_chain_hash
                .as_ref()
                .map_or(Value::Null, |value| Value::String(value.clone())),
        ),
        string("chain_hash", &row.chain_hash),
        string("seal_algorithm", &row.seal_algorithm),
        string("seal_key_id", &row.seal_key_id),
        string("seal", &row.seal),
    ]))
}

fn key_is_durably_revoked(connection: &Connection, key_id: &str) -> Result<bool> {
    connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM tlpx_revocations
               WHERE scope_type = 'AUTHORIZATION_MAC_KEY' AND scope_id = ?1
             )",
            [key_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(db_error)
}

fn verify_envelope_binding(record: &Value, row: &SealedEvidence) -> Result<()> {
    let Value::Object(fields) = record else {
        return Err(Error::authority("evidence record must be an object"));
    };
    let field = |name: &str| {
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    };
    let Some(Value::String(record_type)) = field("record_type") else {
        return Err(Error::authority("evidence record_type is missing"));
    };
    if record_type != &row.record_type {
        return Err(Error::authority("evidence envelope record_type mismatch"));
    }
    let source_field = match record_type.as_str() {
        "tlpx.decision" | "tlpx.evaluation_error" => "receipt_id",
        "tlpx.operator_action" => "receipt_id",
        "tlpx.authorization" => "authorization_id",
        "tlpx.authorization_claim" => "claim_id",
        "tlpx.execution" => "execution_id",
        _ => return Err(Error::authority("unsupported evidence record type")),
    };
    let Some(Value::String(source_id)) = field(source_field) else {
        return Err(Error::authority("evidence source identifier is missing"));
    };
    if source_id != &row.source_id {
        return Err(Error::authority("evidence envelope source mismatch"));
    }
    Ok(())
}

fn verify_coverage(connection: &Connection) -> Result<()> {
    let missing_evaluations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM tlpx_evaluations e
             WHERE NOT EXISTS (
               SELECT 1 FROM tlpx_evidence_outbox o
               WHERE o.source_id = e.receipt_id
                 AND o.record_type = CASE e.outcome_kind
                   WHEN 'DECISION' THEN 'tlpx.decision'
                   ELSE 'tlpx.evaluation_error'
                 END
             )",
            [],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    let missing_authorizations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM tlpx_authorizations a
             WHERE NOT EXISTS (
               SELECT 1 FROM tlpx_evidence_outbox o
               WHERE o.record_type = 'tlpx.authorization'
                 AND o.source_id = a.authorization_id
             )",
            [],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    let missing_claims: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM tlpx_claims c
             WHERE NOT EXISTS (
               SELECT 1 FROM tlpx_evidence_outbox o
               WHERE o.record_type = 'tlpx.authorization_claim'
                 AND o.source_id = c.claim_id
             )",
            [],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    let missing_operator_actions: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM tlpx_operator_actions a
             WHERE NOT EXISTS (
               SELECT 1 FROM tlpx_evidence_outbox o
               WHERE o.record_type = 'tlpx.operator_action'
                 AND o.source_id = a.receipt_id
             )",
            [],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    let missing_terminal_executions: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM tlpx_executions e
             WHERE e.state IN (
               'COMPLETED','FAILED','CANCELLED','LEASE_EXPIRED',
               'COMPLETED_CONFIRMED','FAILED_CONFIRMED','OUTCOME_UNKNOWN_FINAL'
             )
               AND NOT EXISTS (
                 SELECT 1 FROM tlpx_evidence_outbox o
                 WHERE o.record_type = 'tlpx.execution'
                   AND o.source_id = e.execution_id
               )",
            [],
            |row| row.get(0),
        )
        .map_err(db_error)?;
    if missing_evaluations != 0
        || missing_authorizations != 0
        || missing_claims != 0
        || missing_operator_actions != 0
        || missing_terminal_executions != 0
    {
        return Err(Error::authority(
            "authority state and evidence outbox are not fully reconciled",
        ));
    }
    Ok(())
}

fn verify_source(connection: &Connection, row: &SealedEvidence, record: &Value) -> Result<()> {
    match row.record_type.as_str() {
        "tlpx.decision" | "tlpx.evaluation_error" => {
            verify_evaluation_source(connection, row, record)
        }
        "tlpx.operator_action" => verify_operator_action_source(connection, row, record),
        "tlpx.authorization" => verify_authorization_source(connection, row, record),
        "tlpx.authorization_claim" => verify_claim_source(connection, row, record),
        "tlpx.execution" => verify_execution_source(connection, row, record),
        _ => Err(Error::authority("unsupported evidence record type")),
    }
}

#[allow(clippy::type_complexity)]
fn verify_evaluation_source(
    connection: &Connection,
    row: &SealedEvidence,
    record: &Value,
) -> Result<()> {
    let source = connection
        .query_row(
            "SELECT sequence, authenticated_principal, request_id, intent_hash,
                    retry_of_receipt_id, outcome_kind, decision, policy_id, stage,
                    reason_code, reason, retryability, required_condition,
                    policy_bundle_hash, evaluated_at_ms
             FROM tlpx_evaluations WHERE receipt_id = ?1",
            [&row.source_id],
            |source| {
                Ok((
                    source.get::<_, i64>(0)?,
                    source.get::<_, Option<String>>(1)?,
                    source.get::<_, Option<String>>(2)?,
                    source.get::<_, Option<String>>(3)?,
                    source.get::<_, Option<String>>(4)?,
                    source.get::<_, String>(5)?,
                    source.get::<_, Option<String>>(6)?,
                    source.get::<_, Option<String>>(7)?,
                    source.get::<_, Option<String>>(8)?,
                    source.get::<_, String>(9)?,
                    source.get::<_, String>(10)?,
                    source.get::<_, Option<String>>(11)?,
                    source.get::<_, Option<String>>(12)?,
                    source.get::<_, Option<String>>(13)?,
                    source.get::<_, i64>(14)?,
                ))
            },
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| Error::authority("orphan evidence outbox row"))?;

    let is_decision = row.record_type == "tlpx.decision";
    let requester_path: &[&str] = if is_decision {
        &["parties", "requester", "id"]
    } else {
        &["authenticated_requester"]
    };
    ensure_int(record, &["sequence"], source.0)?;
    ensure_optional_string(record, requester_path, source.1.as_deref())?;
    ensure_optional_string(record, &["request_id"], source.2.as_deref())?;
    ensure_optional_string(record, &["intent_hash"], source.3.as_deref())?;
    ensure_optional_string(record, &["retry_of_receipt_id"], source.4.as_deref())?;
    ensure_string(
        record,
        &["record_type"],
        if source.5 == "DECISION" {
            "tlpx.decision"
        } else {
            "tlpx.evaluation_error"
        },
    )?;
    if is_decision {
        ensure_optional_string(record, &["decision"], source.6.as_deref())?;
        ensure_optional_string(record, &["policy_id"], source.7.as_deref())?;
        ensure_optional_string(record, &["policy_bundle_hash"], source.13.as_deref())?;
        ensure_string(record, &["evaluated_at"], &iso8601_from_ms(source.14)?)?;
        ensure_string(record, &["reason_code"], &source.9)?;
    } else {
        ensure_optional_string(record, &["stage"], source.8.as_deref())?;
        ensure_optional_string(record, &["retryability"], source.11.as_deref())?;
        ensure_optional_string(record, &["required_condition"], source.12.as_deref())?;
        ensure_string(record, &["occurred_at"], &iso8601_from_ms(source.14)?)?;
        ensure_string(record, &["error_code"], &source.9)?;
    }
    ensure_string(record, &["reason"], &source.10)?;
    Ok(())
}

#[allow(clippy::type_complexity)]
fn verify_operator_action_source(
    connection: &Connection,
    row: &SealedEvidence,
    record: &Value,
) -> Result<()> {
    let source = connection
        .query_row(
            "SELECT sequence, outcome, actor_id, actor_type, policy_bundle_hash,
                    acted_at_ms, authorized_action_hash, renderer_id, renderer_version
             FROM tlpx_operator_actions WHERE receipt_id = ?1",
            [&row.source_id],
            |source| {
                Ok((
                    source.get::<_, i64>(0)?,
                    source.get::<_, String>(1)?,
                    source.get::<_, String>(2)?,
                    source.get::<_, String>(3)?,
                    source.get::<_, String>(4)?,
                    source.get::<_, i64>(5)?,
                    source.get::<_, Option<String>>(6)?,
                    source.get::<_, Option<String>>(7)?,
                    source.get::<_, Option<String>>(8)?,
                ))
            },
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| Error::authority("orphan evidence outbox row"))?;
    ensure_int(record, &["sequence"], source.0)?;
    ensure_string(record, &["outcome"], &source.1)?;
    ensure_string(record, &["operator", "id"], &source.2)?;
    ensure_string(record, &["operator", "type"], &source.3)?;
    ensure_string(record, &["policy_bundle_hash"], &source.4)?;
    ensure_string(record, &["acted_at"], &iso8601_from_ms(source.5)?)?;
    ensure_optional_string(record, &["authorized_action_hash"], source.6.as_deref())?;
    ensure_optional_string(record, &["renderer_id"], source.7.as_deref())?;
    ensure_optional_string(record, &["renderer_version"], source.8.as_deref())?;
    Ok(())
}

#[allow(clippy::type_complexity)]
fn verify_authorization_source(
    connection: &Connection,
    row: &SealedEvidence,
    record: &Value,
) -> Result<()> {
    let source = connection
        .query_row(
            "SELECT receipt_id, requesting_principal, executing_principal, action, target,
                    intent_hash, authorized_action_hash, action_binding_hash,
                    adapter_id, adapter_version, environment, tenant,
                    authorization_nonce, issued_at_ms, claim_expires_at_ms,
                    execution_lease_ms
             FROM tlpx_authorizations WHERE authorization_id = ?1",
            [&row.source_id],
            |source| {
                Ok((
                    source.get::<_, String>(0)?,
                    source.get::<_, String>(1)?,
                    source.get::<_, String>(2)?,
                    source.get::<_, String>(3)?,
                    source.get::<_, String>(4)?,
                    source.get::<_, String>(5)?,
                    source.get::<_, String>(6)?,
                    source.get::<_, String>(7)?,
                    source.get::<_, String>(8)?,
                    source.get::<_, String>(9)?,
                    source.get::<_, String>(10)?,
                    source.get::<_, String>(11)?,
                    source.get::<_, String>(12)?,
                    source.get::<_, i64>(13)?,
                    source.get::<_, i64>(14)?,
                    source.get::<_, i64>(15)?,
                ))
            },
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| Error::authority("orphan evidence outbox row"))?;
    ensure_string(record, &["receipt_id"], &source.0)?;
    ensure_string(record, &["requesting_principal"], &source.1)?;
    ensure_string(record, &["executing_principal"], &source.2)?;
    ensure_string(record, &["action"], &source.3)?;
    ensure_string(record, &["target"], &source.4)?;
    ensure_string(record, &["intent_hash"], &source.5)?;
    ensure_string(record, &["authorized_action_hash"], &source.6)?;
    ensure_string(record, &["action_binding_hash"], &source.7)?;
    ensure_string(record, &["adapter", "id"], &source.8)?;
    ensure_string(record, &["adapter", "version"], &source.9)?;
    ensure_string(record, &["environment"], &source.10)?;
    ensure_string(record, &["tenant"], &source.11)?;
    ensure_string(record, &["authorization_nonce"], &source.12)?;
    ensure_string(record, &["issued_at"], &iso8601_from_ms(source.13)?)?;
    ensure_string(record, &["claim_expires_at"], &iso8601_from_ms(source.14)?)?;
    let lease_seconds = source
        .15
        .checked_div(1_000)
        .ok_or_else(|| Error::authority("execution lease conversion failed"))?;
    ensure_int(record, &["execution_lease_seconds"], lease_seconds)?;
    ensure_string(
        record,
        &["idempotency_key"],
        &authorization_idempotency_key(&row.source_id, &source.2, &source.6)?,
    )?;
    Ok(())
}

#[allow(clippy::type_complexity)]
fn verify_claim_source(
    connection: &Connection,
    row: &SealedEvidence,
    record: &Value,
) -> Result<()> {
    let source = connection
        .query_row(
            "SELECT authorization_id, receipt_id, sequence, executing_principal,
                    authorized_action_hash, action_binding_hash, executed_action_hash,
                    adapter_id, adapter_version, claimed_at_ms, lease_expires_at_ms
             FROM tlpx_claims WHERE claim_id = ?1",
            [&row.source_id],
            |source| {
                Ok((
                    source.get::<_, String>(0)?,
                    source.get::<_, String>(1)?,
                    source.get::<_, i64>(2)?,
                    source.get::<_, String>(3)?,
                    source.get::<_, String>(4)?,
                    source.get::<_, String>(5)?,
                    source.get::<_, String>(6)?,
                    source.get::<_, String>(7)?,
                    source.get::<_, String>(8)?,
                    source.get::<_, i64>(9)?,
                    source.get::<_, i64>(10)?,
                ))
            },
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| Error::authority("orphan evidence outbox row"))?;
    ensure_string(record, &["authorization_id"], &source.0)?;
    ensure_string(record, &["receipt_id"], &source.1)?;
    ensure_int(record, &["sequence"], source.2)?;
    ensure_string(record, &["executing_principal"], &source.3)?;
    ensure_string(record, &["authorized_action_hash"], &source.4)?;
    ensure_string(record, &["action_binding_hash"], &source.5)?;
    ensure_string(record, &["executed_action_hash"], &source.6)?;
    ensure_string(record, &["adapter", "id"], &source.7)?;
    ensure_string(record, &["adapter", "version"], &source.8)?;
    ensure_string(record, &["claimed_at"], &iso8601_from_ms(source.9)?)?;
    ensure_string(record, &["lease_expires_at"], &iso8601_from_ms(source.10)?)?;
    Ok(())
}

#[allow(clippy::type_complexity)]
fn verify_execution_source(
    connection: &Connection,
    row: &SealedEvidence,
    record: &Value,
) -> Result<()> {
    let source = connection
        .query_row(
            "SELECT claim_id, authorization_id, receipt_id, requesting_principal,
                    executing_principal, intent_hash, authorized_action_hash,
                    executed_action_hash, target, policy_bundle_id,
                    policy_bundle_version, policy_bundle_hash, adapter_id,
                    adapter_version, adapter_principal, adapter_binary_hash,
                    terminal_sequence, started_at_ms, ended_at_ms, state,
                    result_summary, result_hash, external_evidence_reference,
                    cancellation_outcome
             FROM tlpx_executions WHERE execution_id = ?1",
            [&row.source_id],
            |source| {
                Ok((
                    source.get::<_, String>(0)?,
                    source.get::<_, String>(1)?,
                    source.get::<_, String>(2)?,
                    source.get::<_, String>(3)?,
                    source.get::<_, String>(4)?,
                    source.get::<_, String>(5)?,
                    source.get::<_, String>(6)?,
                    source.get::<_, String>(7)?,
                    source.get::<_, String>(8)?,
                    source.get::<_, String>(9)?,
                    source.get::<_, String>(10)?,
                    source.get::<_, String>(11)?,
                    source.get::<_, String>(12)?,
                    source.get::<_, String>(13)?,
                    source.get::<_, Option<String>>(14)?,
                    source.get::<_, Option<String>>(15)?,
                    source.get::<_, Option<i64>>(16)?,
                    source.get::<_, i64>(17)?,
                    source.get::<_, Option<i64>>(18)?,
                    source.get::<_, String>(19)?,
                    source.get::<_, Option<String>>(20)?,
                    source.get::<_, Option<String>>(21)?,
                    source.get::<_, Option<String>>(22)?,
                    source.get::<_, Option<String>>(23)?,
                ))
            },
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| Error::authority("orphan evidence outbox row"))?;
    for (path, expected) in [
        (&["claim_id"][..], source.0.as_str()),
        (&["authorization_id"][..], source.1.as_str()),
        (&["receipt_id"][..], source.2.as_str()),
        (&["requesting_principal"][..], source.3.as_str()),
        (&["executing_principal"][..], source.4.as_str()),
        (&["intent_hash"][..], source.5.as_str()),
        (&["authorized_action_hash"][..], source.6.as_str()),
        (&["executed_action_hash"][..], source.7.as_str()),
        (&["target"][..], source.8.as_str()),
        (&["policy_bundle_id"][..], source.9.as_str()),
        (&["policy_bundle_version"][..], source.10.as_str()),
        (&["policy_bundle_hash"][..], source.11.as_str()),
        (&["adapter", "id"][..], source.12.as_str()),
        (&["adapter", "version"][..], source.13.as_str()),
        (&["state"][..], source.19.as_str()),
    ] {
        ensure_string(record, path, expected)?;
    }
    ensure_optional_string(record, &["adapter_principal"], source.14.as_deref())?;
    ensure_optional_string(record, &["adapter_binary_hash"], source.15.as_deref())?;
    ensure_optional_int(record, &["sequence"], source.16)?;
    ensure_string(record, &["started_at"], &iso8601_from_ms(source.17)?)?;
    ensure_optional_time(record, &["ended_at"], source.18)?;
    ensure_optional_string(record, &["result_summary"], source.20.as_deref())?;
    ensure_optional_string(record, &["result_hash"], source.21.as_deref())?;
    ensure_optional_string(
        record,
        &["external_evidence_reference"],
        source.22.as_deref(),
    )?;
    ensure_optional_string(record, &["cancellation_outcome"], source.23.as_deref())?;
    Ok(())
}

fn record_value<'a>(record: &'a Value, path: &[&str]) -> Result<Option<&'a Value>> {
    let mut current = record;
    for (index, name) in path.iter().enumerate() {
        let Value::Object(fields) = current else {
            return Err(Error::authority("evidence source binding is not an object"));
        };
        let value = fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value);
        let Some(value) = value else {
            return Ok(None);
        };
        if index == path.len() - 1 {
            return Ok(Some(value));
        }
        current = value;
    }
    Ok(Some(current))
}

fn ensure_string(record: &Value, path: &[&str], expected: &str) -> Result<()> {
    if record_value(record, path)? == Some(&Value::String(expected.to_owned())) {
        Ok(())
    } else {
        Err(source_binding_error(path))
    }
}

fn ensure_optional_string(record: &Value, path: &[&str], expected: Option<&str>) -> Result<()> {
    let actual = record_value(record, path)?;
    let matches = match (actual, expected) {
        (None | Some(Value::Null), None) => true,
        (Some(Value::String(actual)), Some(expected)) => actual == expected,
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        Err(source_binding_error(path))
    }
}

fn ensure_int(record: &Value, path: &[&str], expected: i64) -> Result<()> {
    if record_value(record, path)? == Some(&Value::Int(expected)) {
        Ok(())
    } else {
        Err(source_binding_error(path))
    }
}

fn ensure_optional_int(record: &Value, path: &[&str], expected: Option<i64>) -> Result<()> {
    match expected {
        Some(value) => ensure_int(record, path, value),
        None if matches!(record_value(record, path)?, None | Some(Value::Null)) => Ok(()),
        None => Err(source_binding_error(path)),
    }
}

fn ensure_optional_time(record: &Value, path: &[&str], expected: Option<i64>) -> Result<()> {
    match expected {
        Some(value) => ensure_string(record, path, &iso8601_from_ms(value)?),
        None => ensure_optional_string(record, path, None),
    }
}

fn source_binding_error(path: &[&str]) -> Error {
    Error::authority(format!(
        "authority source row and sealed evidence differ at {}",
        path.join(".")
    ))
}

fn row_to_evidence(row: &rusqlite::Row<'_>) -> rusqlite::Result<SealedEvidence> {
    Ok(SealedEvidence {
        outbox_id: row.get(0)?,
        authority_sequence: row.get(1)?,
        ordinal: row.get(2)?,
        record_type: row.get(3)?,
        source_id: row.get(4)?,
        record_json: row.get(5)?,
        record_hash: row.get(6)?,
        previous_chain_hash: row.get(7)?,
        chain_hash: row.get(8)?,
        seal_algorithm: row.get(9)?,
        seal_key_id: row.get(10)?,
        seal: row.get(11)?,
        exported_at_ms: row.get(12)?,
        export_ack_key_id: row.get(13)?,
        export_ack: row.get(14)?,
    })
}

fn compute_chain_hash(
    previous_chain_hash: Option<&str>,
    authority_sequence: i64,
    ordinal: i64,
    record_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CHAIN_HASH_PREFIX);
    hasher.update(previous_chain_hash.unwrap_or(GENESIS).as_bytes());
    hasher.update([0]);
    hasher.update(authority_sequence.to_string().as_bytes());
    hasher.update([0]);
    hasher.update(ordinal.to_string().as_bytes());
    hasher.update([0]);
    hasher.update(record_hash.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn compute_seal(config: &EvidenceConfig, chain_hash: &str) -> Result<KeyProof> {
    config
        .keys
        .sign(KeyPurpose::AuditSealing, &seal_payload(chain_hash))
}

fn verify_seal(config: &EvidenceConfig, key_id: &str, chain_hash: &str, seal: &str) -> Result<()> {
    config.keys.verify(
        key_id,
        KeyPurpose::AuditSealing,
        &seal_payload(chain_hash),
        seal,
    )
}

fn seal_payload(chain_hash: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(SEAL_PREFIX.len() + chain_hash.len());
    payload.extend_from_slice(SEAL_PREFIX);
    payload.extend_from_slice(chain_hash.as_bytes());
    payload
}

fn compute_export_ack(
    config: &EvidenceConfig,
    outbox_id: i64,
    chain_hash: &str,
    exported_at_ms: i64,
) -> Result<KeyProof> {
    config.keys.sign(
        KeyPurpose::AuditSealing,
        &export_ack_payload(outbox_id, chain_hash, exported_at_ms),
    )
}

fn verify_export_ack(
    config: &EvidenceConfig,
    key_id: &str,
    outbox_id: i64,
    chain_hash: &str,
    exported_at_ms: i64,
    proof: &str,
) -> Result<()> {
    config.keys.verify(
        key_id,
        KeyPurpose::AuditSealing,
        &export_ack_payload(outbox_id, chain_hash, exported_at_ms),
        proof,
    )
}

fn export_ack_payload(outbox_id: i64, chain_hash: &str, exported_at_ms: i64) -> Vec<u8> {
    let mut payload = Vec::with_capacity(EXPORT_ACK_PREFIX.len() + chain_hash.len() + 48);
    payload.extend_from_slice(EXPORT_ACK_PREFIX);
    payload.extend_from_slice(outbox_id.to_string().as_bytes());
    payload.push(0);
    payload.extend_from_slice(chain_hash.as_bytes());
    payload.push(0);
    payload.extend_from_slice(exported_at_ms.to_string().as_bytes());
    payload
}

fn prefixed_sha256(prefix: &[u8], payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix);
    hasher.update(payload);
    format!("sha256:{:x}", hasher.finalize())
}

fn string(name: &str, value: &str) -> (String, Value) {
    (name.into(), Value::String(value.into()))
}

fn party(id: &str, kind: &str) -> Value {
    Value::Object(vec![string("id", id), string("type", kind)])
}

fn adapter(id: &str, version: &str) -> Value {
    Value::Object(vec![string("id", id), string("version", version)])
}

pub(crate) fn iso8601_from_ms(milliseconds: i64) -> Result<String> {
    if milliseconds < 0 {
        return Err(Error::authority("evidence timestamp must not be negative"));
    }
    let seconds = milliseconds / 1_000;
    let millis = milliseconds % 1_000;
    let days = seconds / 86_400;
    let seconds_of_day = seconds % 86_400;
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z"
    ))
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

fn db_error(error: rusqlite::Error) -> Error {
    Error::authority(format!("database: {error}"))
}
