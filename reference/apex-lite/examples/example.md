# APEX Lite - Example Flow

This directory demonstrates a minimal APEX Lite evaluation flow.

## Files

- `intent.json`  
  Declared agent intent represented as explicit metadata.

- `policy.yaml`  
  Local, human-defined rules used to evaluate intent.

- `decision.json`  
  Deterministic outcome produced by evaluating intent against policy.

- `intent-high-risk.json`
  Example intent that matches a high-risk approval rule.

- `intent-safe.json`
  Example intent that matches no rules and is allowed.

## What This Example Shows

- Intent is evaluated **before execution**
- Decisions are deterministic and inspectable
- Policy is external to the agent or model
- Execution does not occur unless explicitly allowed
- The included evaluator supports the small rule grammar used in this repository

## Understanding the Decision Model

APEX Lite produces two possible outcomes: **ALLOW** and **REQUIRE_APPROVAL**.

There is no DENY — and that is intentional.

A `REQUIRE_APPROVAL` decision is not a failure. It means the system is working correctly. The agent declared its intent honestly, the policy identified it as something a human should review, and oversight was triggered. This is the ideal outcome for high-risk or sensitive actions.

By design, escalation is rewarded. An agent that accurately reports what it is about to do — even when the action is risky — is behaving exactly as the system intends. There is no incentive to game or obscure intent, because transparency is never penalized.

## What This Example Does Not Show

- How execution is performed
- How approvals are handled
- How agents are coordinated
- How policies are distributed or enforced

Those concerns are intentionally out of scope.
