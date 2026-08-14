//! Small deterministic policy and identity profile for the local authority MVP.
//! This is intentionally exact-match only; ambiguity fails bundle validation.

use crate::error::{Error, Result};
use crate::hash::assert_hash_string;
use crate::types::{AuthorizedAction, Risk, SubmittedIntent};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    RequireApproval,
    Deny,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Allow => "ALLOW",
            Decision::RequireApproval => "REQUIRE_APPROVAL",
            Decision::Deny => "DENY",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationTemplate {
    pub derived_risk: Risk,
    pub capability: String,
    pub resource_scope: Vec<String>,
    pub risk_reasons: Vec<String>,
    pub risk_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEffect {
    pub decision: Decision,
    pub reason_code: String,
    pub authorization: Option<AuthorizationTemplate>,
}

impl PolicyEffect {
    pub fn allow(reason_code: impl Into<String>, authorization: AuthorizationTemplate) -> Self {
        Self {
            decision: Decision::Allow,
            reason_code: reason_code.into(),
            authorization: Some(authorization),
        }
    }

    pub fn deny(reason_code: impl Into<String>) -> Self {
        Self {
            decision: Decision::Deny,
            reason_code: reason_code.into(),
            authorization: None,
        }
    }

    pub fn require_approval(reason_code: impl Into<String>) -> Self {
        Self {
            decision: Decision::RequireApproval,
            reason_code: reason_code.into(),
            authorization: None,
        }
    }

    fn validate(&self) -> Result<()> {
        require_policy_text(&self.reason_code, "reason_code")?;
        let expected_reason = match self.decision {
            Decision::Allow => "POLICY_ALLOW",
            Decision::RequireApproval => "POLICY_REQUIRE_APPROVAL",
            Decision::Deny => "POLICY_DENY",
        };
        if self.reason_code != expected_reason {
            return Err(Error::policy_compile(format!(
                "{} policy effect requires reason code {expected_reason}",
                self.decision.as_str()
            )));
        }
        match (self.decision, &self.authorization) {
            (Decision::Allow, Some(template)) => template.validate(),
            (Decision::Allow, None) => Err(Error::policy_compile(
                "ALLOW requires an authorization template",
            )),
            (_, Some(_)) => Err(Error::policy_compile("non-ALLOW effect cannot authorize")),
            (_, None) => Ok(()),
        }
    }
}

impl AuthorizationTemplate {
    fn validate(&self) -> Result<()> {
        require_policy_text(&self.capability, "capability")?;
        require_policy_text(&self.risk_source, "risk_source")?;
        if self.resource_scope.is_empty() {
            return Err(Error::policy_compile("resource_scope must not be empty"));
        }
        if self.resource_scope.iter().any(|value| value.is_empty()) {
            return Err(Error::policy_compile(
                "resource_scope entries must not be empty",
            ));
        }
        let unique: BTreeSet<_> = self.resource_scope.iter().collect();
        if unique.len() != self.resource_scope.len() {
            return Err(Error::policy_compile(
                "resource_scope entries must be unique",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRule {
    pub id: String,
    pub action: String,
    pub effect: PolicyEffect,
}

#[derive(Debug, Clone)]
pub struct PolicyBundle {
    pub id: String,
    pub version: String,
    pub hash: String,
    pub rules: Vec<PolicyRule>,
    pub default: PolicyEffect,
}

impl PolicyBundle {
    pub fn validate(&self) -> Result<()> {
        require_policy_text(&self.id, "policy.id")?;
        require_policy_text(&self.version, "policy.version")?;
        assert_hash_string(&self.hash).map_err(|_| Error::policy_compile("invalid policy hash"))?;
        self.default.validate()?;

        let mut ids = BTreeSet::new();
        let mut actions = BTreeSet::new();
        for rule in &self.rules {
            require_policy_text(&rule.id, "rule.id")?;
            require_policy_text(&rule.action, "rule.action")?;
            if !ids.insert(rule.id.as_str()) {
                return Err(Error::policy_compile(format!(
                    "duplicate rule id {}",
                    rule.id
                )));
            }
            if !actions.insert(rule.action.as_str()) {
                return Err(Error::policy_compile(format!(
                    "ambiguous duplicate action {}",
                    rule.action
                )));
            }
            rule.effect.validate()?;
        }
        Ok(())
    }

    pub fn evaluate(&self, intent: &SubmittedIntent) -> PolicyEffect {
        let effect = self
            .rules
            .iter()
            .find(|rule| rule.action == intent.action)
            .map_or(&self.default, |rule| &rule.effect);
        let Some(template) = effect.authorization.as_ref() else {
            return effect.clone();
        };
        if intent.requested_capability != template.capability {
            return PolicyEffect::deny("POLICY_CAPABILITY_MISMATCH");
        }
        if !template
            .resource_scope
            .iter()
            .any(|scope| scope == &intent.target)
        {
            return PolicyEffect::deny("POLICY_TARGET_OUT_OF_SCOPE");
        }
        effect.clone()
    }

    pub fn authorization_templates(&self) -> impl Iterator<Item = &AuthorizationTemplate> {
        std::iter::once(&self.default)
            .chain(self.rules.iter().map(|rule| &rule.effect))
            .filter_map(|effect| effect.authorization.as_ref())
    }

    pub fn authorize(
        &self,
        intent: &SubmittedIntent,
        effect: &PolicyEffect,
    ) -> Result<AuthorizedAction> {
        let template = effect
            .authorization
            .as_ref()
            .ok_or_else(|| Error::authority("decision does not carry an authorization template"))?;
        let effective_risk = max_risk(&intent.declared_risk, &template.derived_risk);
        let action = AuthorizedAction {
            requesting_principal: intent.requesting_principal.clone(),
            executing_principal: intent.executing_principal.clone(),
            action: intent.action.clone(),
            target: intent.target.clone(),
            arguments: intent.arguments.clone(),
            environment: intent.environment.clone(),
            tenant: intent.tenant.clone(),
            derived_risk: template.derived_risk.clone(),
            effective_risk,
            risk_reasons: template.risk_reasons.clone(),
            risk_source: template.risk_source.clone(),
            data_classes: intent.data_classes.clone(),
            capability: template.capability.clone(),
            resource_scope: template.resource_scope.clone(),
            payload_hash: intent.payload_hash.clone(),
            artifact_hash: intent.artifact_hash.clone(),
            policy_bundle_hash: self.hash.clone(),
            adapter: intent.adapter.clone(),
        };
        action.validate()?;
        Ok(action)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub id: String,
    pub active: bool,
    pub allowed_actions: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Switchboard {
    principals: BTreeMap<String, Principal>,
}

impl Switchboard {
    pub fn new(principals: Vec<Principal>) -> Result<Self> {
        let mut result = Self::default();
        for principal in principals {
            require_text(&principal.id, "principal.id")?;
            if result
                .principals
                .insert(principal.id.clone(), principal)
                .is_some()
            {
                return Err(Error::authority("duplicate principal"));
            }
        }
        Ok(result)
    }

    pub fn authorize_intent(
        &self,
        authenticated_requester: &str,
        intent: &SubmittedIntent,
    ) -> Result<()> {
        self.authenticate_requester(authenticated_requester, &intent.requesting_principal)?;
        self.require_action(&intent.requesting_principal, &intent.action)?;
        self.require_action(&intent.executing_principal, &intent.action)
    }

    pub fn refusal_for_intent(&self, intent: &SubmittedIntent) -> Option<&'static str> {
        self.action_refusal(&intent.requesting_principal, &intent.action)
            .or_else(|| self.action_refusal(&intent.executing_principal, &intent.action))
    }

    pub fn authenticate_requester(
        &self,
        authenticated_requester: &str,
        proposed_requester: &str,
    ) -> Result<()> {
        if authenticated_requester != proposed_requester {
            return Err(Error::coded(
                "AUTHENTICATION_FAILED",
                "authenticated requester does not match proposed requester",
            ));
        }
        Ok(())
    }

    pub fn authorize_executor(&self, authenticated_executor: &str, action: &str) -> Result<()> {
        self.require_action(authenticated_executor, action)
    }

    fn require_action(&self, principal_id: &str, action: &str) -> Result<()> {
        if let Some(code) = self.action_refusal(principal_id, action) {
            return Err(Error::coded(code, code));
        }
        Ok(())
    }

    fn action_refusal(&self, principal_id: &str, action: &str) -> Option<&'static str> {
        let principal = self.principals.get(principal_id);
        let Some(principal) = principal else {
            return Some("SWITCHBOARD_UNKNOWN_PRINCIPAL");
        };
        if !principal.active {
            return Some("SWITCHBOARD_PRINCIPAL_INACTIVE");
        }
        if !principal
            .allowed_actions
            .iter()
            .any(|allowed| allowed == action)
        {
            return Some("SWITCHBOARD_ACTION_DENIED");
        }
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    adapters: BTreeMap<String, BTreeSet<String>>,
}

impl CapabilityRegistry {
    pub fn new(entries: Vec<(String, Vec<String>)>) -> Result<Self> {
        let mut registry = Self::default();
        for (capability, adapters) in entries {
            require_text(&capability, "capability")?;
            if adapters.is_empty() || adapters.iter().any(|id| id.is_empty()) {
                return Err(Error::authority(
                    "capability must name at least one adapter",
                ));
            }
            if registry
                .adapters
                .insert(capability, adapters.into_iter().collect())
                .is_some()
            {
                return Err(Error::authority("duplicate capability"));
            }
        }
        Ok(registry)
    }

    pub fn covers(&self, capability: &str, adapter_id: &str) -> bool {
        self.adapters
            .get(capability)
            .is_some_and(|adapters| adapters.contains(adapter_id))
    }

    pub fn contains(&self, capability: &str) -> bool {
        self.adapters.contains_key(capability)
    }
}

fn require_text(value: &str, name: &str) -> Result<()> {
    if value.is_empty() {
        return Err(Error::authority(format!("{name} must be non-empty")));
    }
    Ok(())
}

fn require_policy_text(value: &str, name: &str) -> Result<()> {
    if value.is_empty() {
        return Err(Error::policy_compile(format!("{name} must be non-empty")));
    }
    Ok(())
}

fn max_risk(declared: &Risk, derived: &Risk) -> Risk {
    let rank = |risk: &Risk| match risk {
        Risk::Low => 0,
        Risk::Medium => 1,
        Risk::High => 2,
    };
    if rank(declared) >= rank(derived) {
        declared.clone()
    } else {
        derived.clone()
    }
}
