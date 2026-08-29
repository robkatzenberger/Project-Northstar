//! Optional, non-transitive multi-agent handoff preflight for the bounded local profile.
//!
//! A handoff is a fresh authority evaluation whose submitted intent names the
//! authenticated initiating agent as requester and the authenticated receiving
//! agent as executor. It is not an authorization transfer and creates no bearer
//! handoff credential.
//! This wrapper proves co-presentation only for calls routed through it. The
//! ordinary authority evaluation API remains available and authenticates the
//! named executor later at claim, so this module is not universal mediation.

use crate::authority::{Authority, EvaluationOutcome};
use crate::error::{Error, Result};
use crate::local_auth::{AuthenticatedIdentity, LocalRole};
use crate::types::SubmittedIntent;

impl Authority {
    /// Evaluate a fresh two-agent handoff using the authority's trusted clock.
    pub fn evaluate_handoff_authenticated(
        &self,
        requester: &AuthenticatedIdentity,
        executor: &AuthenticatedIdentity,
        intent: &SubmittedIntent,
    ) -> Result<EvaluationOutcome> {
        validate_handoff(requester, executor, intent)?;
        self.evaluate_authenticated(requester, intent)
    }

    /// Deterministic-time seam for the bounded local profile and its tests.
    pub fn evaluate_handoff_authenticated_at(
        &self,
        requester: &AuthenticatedIdentity,
        executor: &AuthenticatedIdentity,
        intent: &SubmittedIntent,
        evaluated_at_ms: i64,
    ) -> Result<EvaluationOutcome> {
        validate_handoff(requester, executor, intent)?;
        self.evaluate_authenticated_at(requester, intent, evaluated_at_ms)
    }
}

fn validate_handoff(
    requester: &AuthenticatedIdentity,
    executor: &AuthenticatedIdentity,
    intent: &SubmittedIntent,
) -> Result<()> {
    requester.require_role(LocalRole::Requester)?;
    executor.require_role(LocalRole::Executor)?;

    if requester.principal_id() == executor.principal_id() {
        return Err(Error::coded(
            "HANDOFF_REQUIRES_DISTINCT_PRINCIPALS",
            "a multi-agent handoff requires distinct requester and executor principals",
        ));
    }
    if intent.requesting_principal != requester.principal_id() {
        return Err(Error::coded(
            "HANDOFF_REQUESTER_MISMATCH",
            "submitted handoff requester does not match the authenticated initiating agent",
        ));
    }
    if intent.executing_principal != executor.principal_id() {
        return Err(Error::coded(
            "HANDOFF_EXECUTOR_MISMATCH",
            "submitted handoff executor does not match the authenticated receiving agent",
        ));
    }
    if intent.retry_of_receipt_id.is_some() {
        return Err(Error::coded(
            "HANDOFF_RETRY_LINK_FORBIDDEN",
            "a handoff must use a fresh intent; transport retries reuse its request_id",
        ));
    }

    Ok(())
}
