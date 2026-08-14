//! TL-PX 0.2 contract types. Distinct objects; no merged convenience envelope.

use crate::error::{Error, Result};
use crate::hash::{authorized_action_hash, executed_action_hash, intent_hash};
use crate::jcs::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Risk {
    Low,
    Medium,
    High,
}

impl Risk {
    pub fn as_str(&self) -> &'static str {
        match self {
            Risk::Low => "low",
            Risk::Medium => "medium",
            Risk::High => "high",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubmittedIntent {
    pub requesting_principal: String,
    pub executing_principal: String,
    pub action: String,
    pub intent_class: String,
    pub target: String,
    pub arguments: Value,
    pub environment: String,
    pub tenant: String,
    pub declared_risk: Risk,
    pub data_classes: Vec<String>,
    pub requested_capability: String,
    pub resource_scope: Vec<String>,
    pub payload_hash: Option<String>,
    pub artifact_hash: Option<String>,
    pub adapter: Adapter,
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthorizedAction {
    pub requesting_principal: String,
    pub executing_principal: String,
    pub action: String,
    pub target: String,
    pub arguments: Value,
    pub environment: String,
    pub tenant: String,
    pub derived_risk: Risk,
    pub effective_risk: Risk,
    pub data_classes: Vec<String>,
    pub capability: String,
    pub resource_scope: Vec<String>,
    pub payload_hash: Option<String>,
    pub artifact_hash: Option<String>,
    pub policy_bundle_hash: String,
    pub adapter: Adapter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutedAction {
    pub executing_principal: String,
    pub action: String,
    pub target: String,
    pub arguments: Value,
    pub environment: String,
    pub tenant: String,
    pub payload_hash: Option<String>,
    pub artifact_hash: Option<String>,
    pub adapter: Adapter,
}

fn hash_opt(h: &Option<String>) -> Value {
    match h {
        Some(s) => Value::String(s.clone()),
        None => Value::Null,
    }
}

fn adapter_value(a: &Adapter) -> Value {
    Value::Object(vec![
        ("id".into(), Value::String(a.id.clone())),
        ("version".into(), Value::String(a.version.clone())),
    ])
}

fn strings(v: &[String]) -> Value {
    Value::Array(v.iter().cloned().map(Value::String).collect())
}

impl SubmittedIntent {
    pub fn to_value(&self) -> Value {
        Value::Object(vec![
            (
                "requesting_principal".into(),
                Value::String(self.requesting_principal.clone()),
            ),
            (
                "executing_principal".into(),
                Value::String(self.executing_principal.clone()),
            ),
            ("action".into(), Value::String(self.action.clone())),
            (
                "intent_class".into(),
                Value::String(self.intent_class.clone()),
            ),
            ("target".into(), Value::String(self.target.clone())),
            ("arguments".into(), self.arguments.clone()),
            (
                "environment".into(),
                Value::String(self.environment.clone()),
            ),
            ("tenant".into(), Value::String(self.tenant.clone())),
            (
                "declared_risk".into(),
                Value::String(self.declared_risk.as_str().into()),
            ),
            ("data_classes".into(), strings(&self.data_classes)),
            (
                "requested_capability".into(),
                Value::String(self.requested_capability.clone()),
            ),
            ("resource_scope".into(), strings(&self.resource_scope)),
            ("payload_hash".into(), hash_opt(&self.payload_hash)),
            ("artifact_hash".into(), hash_opt(&self.artifact_hash)),
            ("adapter".into(), adapter_value(&self.adapter)),
            ("request_id".into(), Value::String(self.request_id.clone())),
        ])
    }

    pub fn intent_hash(&self) -> Result<String> {
        intent_hash(&self.to_value())
    }
}

impl AuthorizedAction {
    pub fn to_value(&self) -> Value {
        Value::Object(vec![
            (
                "requesting_principal".into(),
                Value::String(self.requesting_principal.clone()),
            ),
            (
                "executing_principal".into(),
                Value::String(self.executing_principal.clone()),
            ),
            ("action".into(), Value::String(self.action.clone())),
            ("target".into(), Value::String(self.target.clone())),
            ("arguments".into(), self.arguments.clone()),
            (
                "environment".into(),
                Value::String(self.environment.clone()),
            ),
            ("tenant".into(), Value::String(self.tenant.clone())),
            (
                "derived_risk".into(),
                Value::String(self.derived_risk.as_str().into()),
            ),
            (
                "effective_risk".into(),
                Value::String(self.effective_risk.as_str().into()),
            ),
            ("data_classes".into(), strings(&self.data_classes)),
            ("capability".into(), Value::String(self.capability.clone())),
            ("resource_scope".into(), strings(&self.resource_scope)),
            ("payload_hash".into(), hash_opt(&self.payload_hash)),
            ("artifact_hash".into(), hash_opt(&self.artifact_hash)),
            (
                "policy_bundle_hash".into(),
                Value::String(self.policy_bundle_hash.clone()),
            ),
            ("adapter".into(), adapter_value(&self.adapter)),
        ])
    }

    pub fn authorized_action_hash(&self) -> Result<String> {
        authorized_action_hash(&self.to_value())
    }

    pub fn effective_risk_ok(&self) -> Result<()> {
        let rank = |r: &Risk| match r {
            Risk::Low => 0,
            Risk::Medium => 1,
            Risk::High => 2,
        };
        if rank(&self.effective_risk) < rank(&self.derived_risk) {
            return Err(Error::hash(
                "effective_risk must not be lower than derived_risk",
            ));
        }
        Ok(())
    }
}

impl ExecutedAction {
    pub fn to_value(&self) -> Value {
        Value::Object(vec![
            (
                "executing_principal".into(),
                Value::String(self.executing_principal.clone()),
            ),
            ("action".into(), Value::String(self.action.clone())),
            ("target".into(), Value::String(self.target.clone())),
            ("arguments".into(), self.arguments.clone()),
            (
                "environment".into(),
                Value::String(self.environment.clone()),
            ),
            ("tenant".into(), Value::String(self.tenant.clone())),
            ("payload_hash".into(), hash_opt(&self.payload_hash)),
            ("artifact_hash".into(), hash_opt(&self.artifact_hash)),
            ("adapter".into(), adapter_value(&self.adapter)),
        ])
    }

    pub fn executed_action_hash(&self) -> Result<String> {
        executed_action_hash(&self.to_value())
    }
}
