//! TL-PX 0.2 policy-bundle provenance and deterministic activation.
//!
//! Manifests bind exact-match policy content. Version strings never select a
//! winner implicitly; overlapping active bundles require explicit, hash-bound
//! supersession.

use crate::error::{Error, Result};
use crate::hash::{assert_hash_string, policy_bundle_hash};
use crate::jcs::{canonicalize, parse, Value};
use crate::policy::{AuthorizationTemplate, Decision, PolicyBundle, PolicyEffect};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const EXACT_MATCH_POLICY_CONTENT_TYPE: &str =
    "application/vnd.tlpx.rust-exact-match+json;version=2";

pub const POLICY_PRECEDENCE: [&str; 6] = [
    "EMERGENCY_DENY",
    "TENANT_ENVIRONMENT_RESTRICTION",
    "SWITCHBOARD_SCOPE",
    "BASE_POLICY",
    "ACTION_POLICY",
    "HUMAN_APPROVAL_CONDITION",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyIssuerType {
    Human,
    Machine,
}

impl PolicyIssuerType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Machine => "machine",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "human" => Ok(Self::Human),
            "machine" => Ok(Self::Machine),
            _ => Err(provenance("issuer.type must be human or machine")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyIssuer {
    pub id: String,
    pub kind: PolicyIssuerType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySupersedes {
    pub policy_bundle_id: String,
    pub policy_bundle_version: String,
    pub policy_bundle_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyBundleManifest {
    pub policy_bundle_id: String,
    pub policy_bundle_version: String,
    pub issuer: PolicyIssuer,
    pub content_type: String,
    pub content_hash: String,
    pub activated_at: String,
    pub retired_at: Option<String>,
    pub environment: String,
    pub tenant: String,
    pub precedence: Vec<String>,
    pub default_decision: Decision,
    pub supersedes: Option<PolicySupersedes>,
}

impl PolicyBundleManifest {
    pub fn from_json(text: &str) -> Result<Self> {
        let value = parse(text).map_err(|error| provenance(error.message()))?;
        Self::from_value(&value)
    }

    pub fn from_value(value: &Value) -> Result<Self> {
        let fields = object(value, "policy manifest")?;
        require_keys(
            fields,
            &[
                "manifest_type",
                "standard",
                "standard_version",
                "policy_bundle_id",
                "policy_bundle_version",
                "issuer",
                "content_type",
                "content_hash",
                "activated_at",
                "retired_at",
                "environment",
                "tenant",
                "precedence",
                "default_decision",
            ],
            &["supersedes"],
            "policy manifest",
        )?;
        if string_field(fields, "manifest_type")? != "tlpx.policy_bundle"
            || string_field(fields, "standard")? != "TL-PX"
            || string_field(fields, "standard_version")? != "0.2.0"
        {
            return Err(provenance("unsupported policy manifest stamp"));
        }

        let issuer_fields = object(field(fields, "issuer")?, "issuer")?;
        require_keys(issuer_fields, &["id", "type"], &[], "issuer")?;
        let issuer = PolicyIssuer {
            id: string_field(issuer_fields, "id")?.to_string(),
            kind: PolicyIssuerType::parse(string_field(issuer_fields, "type")?)?,
        };

        let retired_at = match field(fields, "retired_at")? {
            Value::Null => None,
            Value::String(value) => Some(value.clone()),
            _ => return Err(provenance("retired_at must be a canonical instant or null")),
        };
        let precedence = match field(fields, "precedence")? {
            Value::Array(values) => values
                .iter()
                .map(|value| match value {
                    Value::String(value) => Ok(value.clone()),
                    _ => Err(provenance("precedence entries must be strings")),
                })
                .collect::<Result<Vec<_>>>()?,
            _ => return Err(provenance("precedence must be an array")),
        };
        let default_decision = parse_decision(string_field(fields, "default_decision")?)?;
        let supersedes = match optional_field(fields, "supersedes") {
            None => None,
            Some(value) => {
                let values = object(value, "supersedes")?;
                require_keys(
                    values,
                    &[
                        "policy_bundle_id",
                        "policy_bundle_version",
                        "policy_bundle_hash",
                    ],
                    &[],
                    "supersedes",
                )?;
                Some(PolicySupersedes {
                    policy_bundle_id: string_field(values, "policy_bundle_id")?.to_string(),
                    policy_bundle_version: string_field(values, "policy_bundle_version")?
                        .to_string(),
                    policy_bundle_hash: string_field(values, "policy_bundle_hash")?.to_string(),
                })
            }
        };

        let manifest = Self {
            policy_bundle_id: string_field(fields, "policy_bundle_id")?.to_string(),
            policy_bundle_version: string_field(fields, "policy_bundle_version")?.to_string(),
            issuer,
            content_type: string_field(fields, "content_type")?.to_string(),
            content_hash: string_field(fields, "content_hash")?.to_string(),
            activated_at: string_field(fields, "activated_at")?.to_string(),
            retired_at,
            environment: string_field(fields, "environment")?.to_string(),
            tenant: string_field(fields, "tenant")?.to_string(),
            precedence,
            default_decision,
            supersedes,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("policy_bundle_id", self.policy_bundle_id.as_str()),
            ("issuer.id", self.issuer.id.as_str()),
            ("content_type", self.content_type.as_str()),
            ("environment", self.environment.as_str()),
            ("tenant", self.tenant.as_str()),
        ] {
            if value.is_empty() {
                return Err(provenance(format!("{name} must be non-empty")));
            }
        }
        validate_semver(&self.policy_bundle_version)?;
        assert_hash_string(&self.content_hash)
            .map_err(|_| provenance("content_hash must be a canonical sha256 hash"))?;
        let activated = parse_canonical_instant(&self.activated_at)?;
        if let Some(retired_at) = &self.retired_at {
            let retired = parse_canonical_instant(retired_at)?;
            if retired <= activated {
                return Err(provenance("retired_at must be later than activated_at"));
            }
        }
        let expected = POLICY_PRECEDENCE
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        if self.precedence != expected {
            return Err(provenance(
                "policy precedence is missing, reordered, or extended",
            ));
        }
        if let Some(prior) = &self.supersedes {
            if prior.policy_bundle_id.is_empty() {
                return Err(provenance("superseded policy_bundle_id must be non-empty"));
            }
            validate_semver(&prior.policy_bundle_version)?;
            assert_hash_string(&prior.policy_bundle_hash).map_err(|_| {
                provenance("superseded policy_bundle_hash must be a canonical sha256 hash")
            })?;
            if prior.policy_bundle_id == self.policy_bundle_id
                && prior.policy_bundle_version == self.policy_bundle_version
            {
                return Err(provenance("a policy bundle cannot supersede itself"));
            }
        }
        Ok(())
    }

    pub fn to_value(&self) -> Value {
        let mut fields = vec![
            string("manifest_type", "tlpx.policy_bundle"),
            string("standard", "TL-PX"),
            string("standard_version", "0.2.0"),
            string("policy_bundle_id", &self.policy_bundle_id),
            string("policy_bundle_version", &self.policy_bundle_version),
            (
                "issuer".into(),
                Value::Object(vec![
                    string("id", &self.issuer.id),
                    string("type", self.issuer.kind.as_str()),
                ]),
            ),
            string("content_type", &self.content_type),
            string("content_hash", &self.content_hash),
            string("activated_at", &self.activated_at),
            (
                "retired_at".into(),
                self.retired_at
                    .as_ref()
                    .map_or(Value::Null, |value| Value::String(value.clone())),
            ),
            string("environment", &self.environment),
            string("tenant", &self.tenant),
            (
                "precedence".into(),
                Value::Array(self.precedence.iter().cloned().map(Value::String).collect()),
            ),
            string("default_decision", self.default_decision.as_str()),
        ];
        if let Some(prior) = &self.supersedes {
            fields.push((
                "supersedes".into(),
                Value::Object(vec![
                    string("policy_bundle_id", &prior.policy_bundle_id),
                    string("policy_bundle_version", &prior.policy_bundle_version),
                    string("policy_bundle_hash", &prior.policy_bundle_hash),
                ]),
            ));
        }
        Value::Object(fields)
    }

    pub fn manifest_hash(&self) -> Result<String> {
        self.validate()?;
        policy_bundle_hash(&self.to_value())
    }

    fn activation_ms(&self) -> Result<i64> {
        parse_canonical_instant(&self.activated_at)
    }

    fn retirement_ms(&self) -> Result<Option<i64>> {
        self.retired_at
            .as_deref()
            .map(parse_canonical_instant)
            .transpose()
    }
}

#[derive(Debug, Clone)]
pub struct ConfiguredPolicyBundle {
    pub manifest: PolicyBundleManifest,
    pub policy: PolicyBundle,
}

impl ConfiguredPolicyBundle {
    pub fn validate(&self) -> Result<()> {
        self.manifest.validate()?;
        if self.manifest.content_type != EXACT_MATCH_POLICY_CONTENT_TYPE {
            return Err(provenance(format!(
                "unsupported Rust authority policy content type {}",
                self.manifest.content_type
            )));
        }
        self.policy.validate()?;
        if self.manifest.default_decision != self.policy.default.decision {
            return Err(provenance(
                "manifest default_decision does not match exact-match policy content",
            ));
        }
        let computed = exact_match_policy_content_hash(&self.policy)?;
        if self.manifest.content_hash != computed {
            return Err(provenance("policy manifest content_hash mismatch"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct PolicyCatalog {
    pub bundles: Vec<ConfiguredPolicyBundle>,
}

#[derive(Debug)]
pub struct SelectedPolicy<'a> {
    pub bundle: &'a ConfiguredPolicyBundle,
    pub policy_bundle_hash: String,
}

impl PolicyCatalog {
    pub fn new(bundles: Vec<ConfiguredPolicyBundle>) -> Self {
        Self { bundles }
    }

    pub fn validate(&self) -> Result<()> {
        if self.bundles.is_empty() {
            return Err(Error::coded(
                "POLICY_UNAVAILABLE",
                "no policy bundles configured",
            ));
        }
        let by_key = self.index()?;
        for index in 0..self.bundles.len() {
            self.supersession_ancestors(index, &by_key)?;
        }
        Ok(())
    }

    pub fn select(
        &self,
        environment: &str,
        tenant: &str,
        trusted_time_ms: i64,
    ) -> Result<SelectedPolicy<'_>> {
        if trusted_time_ms < 0 {
            return Err(provenance(
                "trusted policy-selection time must not be negative",
            ));
        }
        self.validate()?;
        let by_key = self.index()?;
        let mut active = Vec::new();
        for (index, configured) in self.bundles.iter().enumerate() {
            let manifest = &configured.manifest;
            if manifest.environment != environment || manifest.tenant != tenant {
                continue;
            }
            let activated = manifest.activation_ms()?;
            let retired = manifest.retirement_ms()?;
            if activated <= trusted_time_ms && retired.map_or(true, |value| trusted_time_ms < value)
            {
                active.push(index);
            }
        }
        if active.is_empty() {
            return Err(Error::coded(
                "POLICY_UNAVAILABLE",
                "no active policy for exact tenant/environment",
            ));
        }

        let selected = if active.len() == 1 {
            active[0]
        } else {
            let active_set = active.iter().copied().collect::<BTreeSet<_>>();
            let mut winners = Vec::new();
            for candidate in active.iter().copied() {
                let ancestors = self.supersession_ancestors(candidate, &by_key)?;
                if active_set
                    .iter()
                    .all(|index| *index == candidate || ancestors.contains(index))
                {
                    winners.push(candidate);
                }
            }
            if winners.len() != 1 {
                return Err(Error::coded(
                    "POLICY_PRECEDENCE_AMBIGUOUS",
                    "overlapping active bundles require one explicit supersession winner",
                ));
            }
            winners[0]
        };
        let bundle = &self.bundles[selected];
        Ok(SelectedPolicy {
            policy_bundle_hash: bundle.manifest.manifest_hash()?,
            bundle,
        })
    }

    pub fn authorization_templates(&self) -> impl Iterator<Item = &AuthorizationTemplate> {
        self.bundles
            .iter()
            .flat_map(|bundle| bundle.policy.authorization_templates())
    }

    fn index(&self) -> Result<BTreeMap<String, usize>> {
        let mut by_key = BTreeMap::new();
        for (index, bundle) in self.bundles.iter().enumerate() {
            bundle.validate()?;
            let key = bundle_key(&bundle.manifest);
            if by_key.insert(key.clone(), index).is_some() {
                return Err(provenance(format!("duplicate policy identity {key}")));
            }
        }
        Ok(by_key)
    }

    fn supersession_ancestors(
        &self,
        candidate: usize,
        by_key: &BTreeMap<String, usize>,
    ) -> Result<BTreeSet<usize>> {
        let mut ancestors = BTreeSet::new();
        let mut edges = Vec::new();
        let mut cursor = candidate;
        while let Some(prior_ref) = &self.bundles[cursor].manifest.supersedes {
            let key = format!(
                "{}@{}",
                prior_ref.policy_bundle_id, prior_ref.policy_bundle_version
            );
            let prior = *by_key
                .get(&key)
                .ok_or_else(|| provenance(format!("superseded manifest {key} is unavailable")))?;
            if !ancestors.insert(prior) {
                return Err(provenance("policy supersession cycle"));
            }
            edges.push((cursor, prior));
            cursor = prior;
        }
        for (successor, prior) in edges {
            let successor_manifest = &self.bundles[successor].manifest;
            let prior_manifest = &self.bundles[prior].manifest;
            let prior_ref = successor_manifest
                .supersedes
                .as_ref()
                .ok_or_else(|| provenance("supersession edge disappeared"))?;
            if successor_manifest.environment != prior_manifest.environment
                || successor_manifest.tenant != prior_manifest.tenant
            {
                return Err(provenance(
                    "supersession must preserve exact tenant/environment scope",
                ));
            }
            if successor_manifest.activation_ms()? <= prior_manifest.activation_ms()? {
                return Err(provenance(
                    "a successor must activate after its predecessor",
                ));
            }
            if prior_ref.policy_bundle_hash != prior_manifest.manifest_hash()? {
                return Err(provenance("supersession predecessor hash mismatch"));
            }
        }
        Ok(ancestors)
    }
}

pub fn exact_match_policy_content_hash(policy: &PolicyBundle) -> Result<String> {
    policy.validate()?;
    let canonical = canonicalize(&policy_content_value(policy))?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_str().as_bytes());
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn policy_content_value(policy: &PolicyBundle) -> Value {
    Value::Object(vec![
        string("profile", "tlpx.rust_exact_match.v2"),
        (
            "rules".into(),
            Value::Array(
                policy
                    .rules
                    .iter()
                    .map(|rule| {
                        Value::Object(vec![
                            string("id", &rule.id),
                            string("action", &rule.action),
                            ("effect".into(), effect_value(&rule.effect)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("default".into(), effect_value(&policy.default)),
    ])
}

fn effect_value(effect: &PolicyEffect) -> Value {
    Value::Object(vec![
        string("decision", effect.decision.as_str()),
        string("reason_code", &effect.reason_code),
        (
            "authorization".into(),
            effect
                .authorization
                .as_ref()
                .map_or(Value::Null, |template| {
                    Value::Object(vec![
                        string("derived_risk", template.derived_risk.as_str()),
                        string("capability", &template.capability),
                        (
                            "resource_scope".into(),
                            Value::Array(
                                template
                                    .resource_scope
                                    .iter()
                                    .cloned()
                                    .map(Value::String)
                                    .collect(),
                            ),
                        ),
                        (
                            "risk_reasons".into(),
                            Value::Array(
                                template
                                    .risk_reasons
                                    .iter()
                                    .cloned()
                                    .map(Value::String)
                                    .collect(),
                            ),
                        ),
                        string("risk_source", &template.risk_source),
                    ])
                }),
        ),
        (
            "approval_route".into(),
            effect.approval_route.as_ref().map_or(Value::Null, |route| {
                Value::Array(route.iter().cloned().map(Value::String).collect())
            }),
        ),
    ])
}

fn bundle_key(manifest: &PolicyBundleManifest) -> String {
    format!(
        "{}@{}",
        manifest.policy_bundle_id, manifest.policy_bundle_version
    )
}

fn parse_decision(value: &str) -> Result<Decision> {
    match value {
        "ALLOW" => Ok(Decision::Allow),
        "REQUIRE_APPROVAL" => Ok(Decision::RequireApproval),
        "DENY" => Ok(Decision::Deny),
        _ => Err(provenance("default_decision is invalid")),
    }
}

fn validate_semver(value: &str) -> Result<()> {
    let (without_build, build) = value
        .split_once('+')
        .map_or((value, None), |(left, right)| (left, Some(right)));
    if value.matches('+').count() > 1 {
        return Err(provenance(
            "policy_bundle_version is not strict semantic version",
        ));
    }
    let (core, prerelease) = without_build
        .split_once('-')
        .map_or((without_build, None), |(left, right)| (left, Some(right)));
    let parts = core.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
        || !prerelease.map_or(true, valid_semver_suffix)
        || !build.map_or(true, valid_semver_suffix)
    {
        return Err(provenance(
            "policy_bundle_version is not strict semantic version",
        ));
    }
    Ok(())
}

fn valid_semver_suffix(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn parse_canonical_instant(value: &str) -> Result<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 24
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'.'
        || bytes[23] != b'Z'
    {
        return Err(provenance(
            "policy time must be canonical UTC ISO-8601 with milliseconds",
        ));
    }
    let year = digits(bytes, 0, 4)?;
    let month = digits(bytes, 5, 2)?;
    let day = digits(bytes, 8, 2)?;
    let hour = digits(bytes, 11, 2)?;
    let minute = digits(bytes, 14, 2)?;
    let second = digits(bytes, 17, 2)?;
    let millisecond = digits(bytes, 20, 3)?;
    if !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(provenance("policy time is not a valid calendar instant"));
    }
    let days = days_from_civil(year, month, day);
    days.checked_mul(86_400_000)
        .and_then(|value| value.checked_add(hour * 3_600_000))
        .and_then(|value| value.checked_add(minute * 60_000))
        .and_then(|value| value.checked_add(second * 1_000))
        .and_then(|value| value.checked_add(millisecond))
        .ok_or_else(|| provenance("policy time is outside the supported range"))
}

fn digits(bytes: &[u8], start: usize, length: usize) -> Result<i64> {
    let mut value = 0_i64;
    for byte in &bytes[start..start + length] {
        if !byte.is_ascii_digit() {
            return Err(provenance(
                "policy time must be canonical UTC ISO-8601 with milliseconds",
            ));
        }
        value = value * 10 + i64::from(byte - b'0');
    }
    Ok(value)
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(mut year: i64, month: i64, day: i64) -> i64 {
    year -= i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn object<'a>(value: &'a Value, name: &str) -> Result<&'a [(String, Value)]> {
    match value {
        Value::Object(fields) => Ok(fields),
        _ => Err(provenance(format!("{name} must be an object"))),
    }
}

fn require_keys(
    fields: &[(String, Value)],
    required: &[&str],
    optional: &[&str],
    name: &str,
) -> Result<()> {
    for required_key in required {
        if !fields.iter().any(|(key, _)| key == required_key) {
            return Err(provenance(format!("{name}.{required_key} is required")));
        }
    }
    if let Some((unknown, _)) = fields.iter().find(|(key, _)| {
        !required.iter().any(|known| key == known) && !optional.iter().any(|known| key == known)
    }) {
        return Err(provenance(format!("{name}.{unknown} is not allowed")));
    }
    Ok(())
}

fn field<'a>(fields: &'a [(String, Value)], name: &str) -> Result<&'a Value> {
    optional_field(fields, name).ok_or_else(|| provenance(format!("{name} is required")))
}

fn optional_field<'a>(fields: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn string_field<'a>(fields: &'a [(String, Value)], name: &str) -> Result<&'a str> {
    match field(fields, name)? {
        Value::String(value) => Ok(value),
        _ => Err(provenance(format!("{name} must be a string"))),
    }
}

fn string(name: &str, value: &str) -> (String, Value) {
    (name.into(), Value::String(value.into()))
}

fn provenance(message: impl Into<String>) -> Error {
    Error::coded("POLICY_PROVENANCE_INVALID", message)
}
