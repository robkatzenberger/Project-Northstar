//! TL-PX 0.2 contract types. Distinct objects; no merged convenience envelope.

use crate::error::{Error, Result};
use crate::hash::{assert_hash_string, authorized_action_hash, executed_action_hash, intent_hash};
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

    fn rank(&self) -> u8 {
        match self {
            Risk::Low => 0,
            Risk::Medium => 1,
            Risk::High => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    pub id: String,
    pub version: String,
}

/// Shared operation projection of Authorized Action and Executed Action.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionBinding {
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
    pub risk_reasons: Vec<String>,
    pub risk_source: String,
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

fn require_id(s: &str, name: &str) -> Result<()> {
    if s.is_empty() {
        return Err(Error::hash(format!("{name} must be non-empty")));
    }
    Ok(())
}

fn require_object(v: &Value, name: &str) -> Result<()> {
    match v {
        Value::Object(_) => v.validate(),
        _ => Err(Error::hash(format!("{name} must be an object"))),
    }
}

fn require_hash(s: &str, name: &str) -> Result<()> {
    assert_hash_string(s).map_err(|_| Error::hash(format!("{name} is not a sha256 hash string")))
}

fn require_hash_opt(h: &Option<String>, name: &str) -> Result<()> {
    match h {
        Some(s) => require_hash(s, name),
        None => Ok(()),
    }
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

impl Adapter {
    pub fn validate(&self) -> Result<()> {
        require_id(&self.id, "adapter.id")?;
        require_id(&self.version, "adapter.version")
    }
}

impl ActionBinding {
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

    pub fn validate(&self) -> Result<()> {
        require_id(&self.executing_principal, "executing_principal")?;
        require_id(&self.action, "action")?;
        require_id(&self.target, "target")?;
        require_object(&self.arguments, "arguments")?;
        require_id(&self.environment, "environment")?;
        require_id(&self.tenant, "tenant")?;
        require_hash_opt(&self.payload_hash, "payload_hash")?;
        require_hash_opt(&self.artifact_hash, "artifact_hash")?;
        self.adapter.validate()
    }

    /// Binding digest: executed-action domain over the shared projection.
    pub fn binding_hash(&self) -> Result<String> {
        self.validate()?;
        executed_action_hash(&self.to_value())
    }
}

impl SubmittedIntent {
    pub fn validate(&self) -> Result<()> {
        require_id(&self.requesting_principal, "requesting_principal")?;
        require_id(&self.executing_principal, "executing_principal")?;
        require_id(&self.action, "action")?;
        require_id(&self.intent_class, "intent_class")?;
        require_id(&self.target, "target")?;
        require_object(&self.arguments, "arguments")?;
        require_id(&self.environment, "environment")?;
        require_id(&self.tenant, "tenant")?;
        require_id(&self.requested_capability, "requested_capability")?;
        require_id(&self.request_id, "request_id")?;
        require_hash_opt(&self.payload_hash, "payload_hash")?;
        require_hash_opt(&self.artifact_hash, "artifact_hash")?;
        self.adapter.validate()
    }

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
        self.validate()?;
        intent_hash(&self.to_value())
    }
}

impl AuthorizedAction {
    pub fn binding(&self) -> ActionBinding {
        ActionBinding {
            executing_principal: self.executing_principal.clone(),
            action: self.action.clone(),
            target: self.target.clone(),
            arguments: self.arguments.clone(),
            environment: self.environment.clone(),
            tenant: self.tenant.clone(),
            payload_hash: self.payload_hash.clone(),
            artifact_hash: self.artifact_hash.clone(),
            adapter: self.adapter.clone(),
        }
    }

    pub fn effective_risk_ok(&self) -> Result<()> {
        if self.effective_risk.rank() < self.derived_risk.rank() {
            return Err(Error::hash(
                "effective_risk must not be lower than derived_risk",
            ));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        require_id(&self.requesting_principal, "requesting_principal")?;
        require_id(&self.capability, "capability")?;
        require_id(&self.risk_source, "risk_source")?;
        require_hash(&self.policy_bundle_hash, "policy_bundle_hash")?;
        self.effective_risk_ok()?;
        self.binding().validate()
    }

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
            ("risk_reasons".into(), strings(&self.risk_reasons)),
            (
                "risk_source".into(),
                Value::String(self.risk_source.clone()),
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
        self.validate()?;
        authorized_action_hash(&self.to_value())
    }

    pub fn binding_hash(&self) -> Result<String> {
        self.validate()?;
        self.binding().binding_hash()
    }
}

impl ExecutedAction {
    pub fn binding(&self) -> ActionBinding {
        ActionBinding {
            executing_principal: self.executing_principal.clone(),
            action: self.action.clone(),
            target: self.target.clone(),
            arguments: self.arguments.clone(),
            environment: self.environment.clone(),
            tenant: self.tenant.clone(),
            payload_hash: self.payload_hash.clone(),
            artifact_hash: self.artifact_hash.clone(),
            adapter: self.adapter.clone(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.binding().validate()
    }

    pub fn to_value(&self) -> Value {
        self.binding().to_value()
    }

    pub fn executed_action_hash(&self) -> Result<String> {
        self.validate()?;
        self.binding().binding_hash()
    }
}

/// PEP check: same Action Binding, same executed-action domain digest.
pub fn bindings_match(authorized: &AuthorizedAction, executed: &ExecutedAction) -> Result<bool> {
    Ok(authorized.binding_hash()? == executed.executed_action_hash()?)
}
