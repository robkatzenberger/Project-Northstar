//! Schema-shaped TL-PX 0.2 evidence plus a sealed durable local outbox.
//!
//! The TL-PX record JSON remains independent of the local storage envelope.
//! Chain hashes and HMAC seals are outbox columns, not private record fields.

use crate::authority::{ClaimRecord, IssuedAuthorization, Retryability};
use crate::error::{Error, Result};
use crate::jcs::{canonicalize, parse, Canonical, Value};
use crate::policy::Decision;
use hmac::{Hmac, Mac};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use std::fmt;

type HmacSha256 = Hmac<Sha256>;

const RECORD_HASH_PREFIX: &[u8] = b"northstar:evidence-record:v1\0";
const CHAIN_HASH_PREFIX: &[u8] = b"northstar:evidence-chain:v1\0";
const SEAL_PREFIX: &[u8] = b"northstar:evidence-seal:v1\0";
const IDEMPOTENCY_PREFIX: &[u8] = b"northstar:idempotency-key:v1\0";
const GENESIS: &str = "GENESIS";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyType {
    Human,
    Machine,
}

impl PartyType {
    fn as_str(self) -> &'static str {
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
    pub seal_key_id: String,
    pub seal_key: Vec<u8>,
}

impl fmt::Debug for EvidenceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvidenceConfig")
            .field("evaluator_id", &self.evaluator_id)
            .field("router_id", &self.router_id)
            .field("requester_type", &self.requester_type)
            .field("seal_key_id", &self.seal_key_id)
            .field("seal_key", &"[REDACTED]")
            .finish()
    }
}

impl EvidenceConfig {
    pub(crate) fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("evaluator_id", self.evaluator_id.as_str()),
            ("router_id", self.router_id.as_str()),
            ("seal_key_id", self.seal_key_id.as_str()),
        ] {
            if value.is_empty() {
                return Err(Error::authority(format!("{name} must be non-empty")));
            }
        }
        if self.seal_key.len() < 32 {
            return Err(Error::authority(
                "audit sealing key must contain at least 32 bytes",
            ));
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

pub(crate) fn authorization_record(issued: &IssuedAuthorization) -> Result<Canonical> {
    let lease_seconds = issued
        .execution_lease_ms
        .checked_div(1_000)
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| Error::authority("execution lease is not whole positive seconds"))?;
    let idempotency_key = prefixed_sha256(
        IDEMPOTENCY_PREFIX,
        canonicalize(&Value::Object(vec![
            string("authenticated_requester", &issued.requesting_principal),
            string("request_id", &issued.request_id),
        ]))?
        .as_str()
        .as_bytes(),
    );
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
        string("idempotency_key", &idempotency_key),
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
                config.seal_key_id,
                seal,
            ],
        )
        .map_err(db_error)?;
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
                    seal_algorithm, seal_key_id, seal, exported_at_ms
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
    let updated = transaction
        .execute(
            "UPDATE tlpx_evidence_outbox
             SET exported_at_ms = ?1
             WHERE outbox_id = ?2 AND chain_hash = ?3 AND exported_at_ms IS NULL
               AND outbox_id = (
                 SELECT MIN(outbox_id) FROM tlpx_evidence_outbox WHERE exported_at_ms IS NULL
               )",
            params![exported_at_ms, outbox_id, expected_chain_hash],
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
            "evidence export acknowledgements must preserve outbox order",
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
                    seal_algorithm, seal_key_id, seal, exported_at_ms
             FROM tlpx_evidence_outbox ORDER BY outbox_id",
        )
        .map_err(db_error)?;
    let rows = statement
        .query_map([], row_to_evidence)
        .map_err(db_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let mut expected_previous: Option<String> = None;
    let mut pending_count = 0_i64;
    let mut exported_count = 0_i64;
    for row in &rows {
        if row.previous_chain_hash != expected_previous {
            return Err(Error::authority("evidence chain predecessor mismatch"));
        }
        if row.seal_algorithm != "HMAC-SHA256" || row.seal_key_id != config.seal_key_id {
            return Err(Error::authority("evidence seal metadata mismatch"));
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
        verify_seal(config, &row.chain_hash, &row.seal)?;
        verify_envelope_binding(&parsed, row)?;
        verify_source(connection, row)?;
        expected_previous = Some(row.chain_hash.clone());
        if row.exported_at_ms.is_some() {
            exported_count += 1;
        } else {
            pending_count += 1;
        }
    }
    verify_coverage(connection)?;
    Ok(EvidenceReconciliation {
        total: i64::try_from(rows.len()).map_err(|_| Error::authority("outbox count overflow"))?,
        pending: pending_count,
        exported: exported_count,
        last_chain_hash: expected_previous,
    })
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
        "tlpx.authorization" => "authorization_id",
        "tlpx.authorization_claim" => "claim_id",
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
    if missing_evaluations != 0 || missing_authorizations != 0 || missing_claims != 0 {
        return Err(Error::authority(
            "authority state and evidence outbox are not fully reconciled",
        ));
    }
    Ok(())
}

fn verify_source(connection: &Connection, row: &SealedEvidence) -> Result<()> {
    let (table, column) = match row.record_type.as_str() {
        "tlpx.decision" | "tlpx.evaluation_error" => ("tlpx_evaluations", "receipt_id"),
        "tlpx.authorization" => ("tlpx_authorizations", "authorization_id"),
        "tlpx.authorization_claim" => ("tlpx_claims", "claim_id"),
        _ => return Err(Error::authority("unsupported evidence record type")),
    };
    let sql = format!("SELECT 1 FROM {table} WHERE {column} = ?1");
    let exists = connection
        .query_row(&sql, [&row.source_id], |_| Ok(()))
        .optional()
        .map_err(db_error)?
        .is_some();
    if !exists {
        return Err(Error::authority("orphan evidence outbox row"));
    }
    Ok(())
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

fn compute_seal(config: &EvidenceConfig, chain_hash: &str) -> Result<String> {
    let mut mac = HmacSha256::new_from_slice(&config.seal_key)
        .map_err(|_| Error::authority("invalid audit sealing key"))?;
    mac.update(SEAL_PREFIX);
    mac.update(chain_hash.as_bytes());
    Ok(format!("hmac-sha256:{}", hex(&mac.finalize().into_bytes())))
}

fn verify_seal(config: &EvidenceConfig, chain_hash: &str, seal: &str) -> Result<()> {
    let encoded = seal
        .strip_prefix("hmac-sha256:")
        .ok_or_else(|| Error::authority("unsupported evidence seal format"))?;
    let bytes = decode_hex_32(encoded)?;
    let mut mac = HmacSha256::new_from_slice(&config.seal_key)
        .map_err(|_| Error::authority("invalid audit sealing key"))?;
    mac.update(SEAL_PREFIX);
    mac.update(chain_hash.as_bytes());
    mac.verify_slice(&bytes)
        .map_err(|_| Error::authority("evidence HMAC verification failed"))
}

fn prefixed_sha256(prefix: &[u8], payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix);
    hasher.update(payload);
    format!("sha256:{:x}", hasher.finalize())
}

fn decode_hex_32(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64 {
        return Err(Error::authority(
            "evidence seal must contain 64 hex characters",
        ));
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| Error::authority("evidence seal contains invalid hex"))?;
    }
    Ok(bytes)
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
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
