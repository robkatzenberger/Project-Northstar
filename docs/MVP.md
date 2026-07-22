# Glass MVP — what we built (v0.1)

**Status:** local prototype for critique  
**Location:** `~/projects/northstar`  
**Not on GitHub**

This document is the handoff brief for the product owner. It is **not** a legal claim set and does not describe patent claims.

---

## Product thesis (as implemented)

A **human + machine trust layer** where:

1. **Intent is declared before execution** (Prism-compatible signal).
2. **Glass evaluates** that intent against deterministic policy.
3. **Humans authorize** when policy escalates.
4. **Execution is recorded** only when authorized (or explicitly blocked).
5. If something goes wrong, an **accountability report** shows which humans and machines were on the chain — declarer, evaluator, authorizer, executor.

Accountability here means **inspectable evidence**, not automatic legal liability.

---

## Scope

### In

| Capability | Notes |
| --- | --- |
| Prism-like signal minting | `prism_v0.1` fields + Glass extensions (`actor_type`, action, risk, …) |
| Deterministic policy eval | APEX-Lite-compatible YAML rules |
| Decisions | `ALLOW` / `REQUIRE_APPROVAL` (`ALLOW_OR_ESCALATE`) |
| Human operator resolve | `APPROVE` / `REJECT` with operator id |
| Execution records | `EXECUTED` / `BLOCKED` / `FAILED` with hard guard on unauthorized execute |
| Append-only JSONL audit | Local file under `var/` |
| Accountability analysis | Findings for human + machine surfaces; incident replay |
| CLI + demo + tests | Zero npm dependencies |

### Out (deliberately)

- Signed auth tokens / crypto
- Multi-tenant SaaS, cloud deploy
- Real agent framework adapters
- LLM-based risk scoring
- SMS / email notifications
- Full UI console
- Patent claim language or filing strategy
- Anything pushed to GitHub

---

## Trust chain model

```text
DECLARER (human|machine)
   │  Prism signal / intent
   ▼
EVALUATOR (machine: glass)
   │  decision receipt
   ▼
AUTHORIZER (human, if escalated)
   │  approve | reject
   ▼
EXECUTOR (human|machine)
   │  executed | blocked | failed
   ▼
AUDIT + optional INCIDENT → accountability report
```

Every step can name **who** (id) and **what kind of party** (human vs machine).

---

## Quick start

```bash
cd ~/projects/northstar

# tests
npm test

# narrative demo (writes var/demo-audit.jsonl)
npm run demo

# manual CLI
node bin/glass.mjs evaluate examples/intent-pii-email.json config/policy.yaml --log var/audit.jsonl
# copy receipt_id from output, then:
node bin/glass.mjs approve <receipt_id> --operator human.ops.alex --log var/audit.jsonl
node bin/glass.mjs execute <receipt_id> --executor runtime.mailer --status EXECUTED --log var/audit.jsonl
node bin/glass.mjs incident <receipt_id> --log var/audit.jsonl --why "Wrong file attached"
node bin/glass.mjs chain <receipt_id> --log var/audit.jsonl
```

---

## Demo scenarios

1. **Safe auto-allow** — low-risk summarize → ALLOW → EXECUTED (machine-only happy path).
2. **HITL + incident** — PII email → REQUIRE_APPROVAL → human APPROVE → EXECUTED → incident report names human authorizer + machine declarer/executor.
3. **Reject** — fund transfer → human REJECT → BLOCKED.

---

## Design choices to challenge

1. **No DENY path** — inherits APEX-Lite “escalate not hard-deny” philosophy; REJECT only after human review.
2. **Glass extensions on Prism** — enrichment lives under `signal.glass` so core Prism stays metadata-light.
3. **Unauthorized execute throws** — runtime must not claim EXECUTED without AUTHORIZED; real agents could still bypass if they ignore the gate (accountability flags missing decision / mismatch).
4. **Heuristic incident attribution** — rules-based surfaces for demo; not a courtroom model.
5. **Policy language** — tiny expression subset via `new Function` + `with(intent)` (same spirit as APEX-Lite). Fine for local MVP; harden later.

---

## Mapping to public Trust Layer story

| Public name | MVP module |
| --- | --- |
| Prism | `src/prism.mjs` |
| Glass / gate | `src/glass.mjs` + `src/policy.mjs` |
| Audit | `src/audit.mjs` |
| Accountability (your emphasis) | `src/accountability.mjs` |
| APEX-Lite lineage | policy format + ALLOW / REQUIRE_APPROVAL |

Local reference still at `~/APEX-Lite`.

---

## Patent / public materials note

Agents searched public web sources. Treat as **research notes only, not legal advice**:

- Public product materials describe Prism + Glass/APEX-style pre-execution verification.
- A specific published patent PDF matching your personal filing was **not confidently identified** from open search alone in this session.
- You mentioned **patent-pending** status and additional docs on another machine — those should be uploaded before we harden product language or public claims.
- MVP wording stays product/engineering focused (“trust layer”, “accountability evidence”, “pre-execution checkpoint”) and avoids claiming legal exclusivity.

When you share filings or internal docs, we can align naming and implementation boundaries without over-disclosing.

---

## Suggested next refinements (after your review)

1. Upload your additional docs → diff against this MVP model.
2. Lock vocabulary: Glass vs APEX vs Trust Layer product names.
3. Decide: port forward from APEX-Lite JS vs this clean-room TS/JS MVP.
4. Add signed decision receipts (even local HMAC) if chain-of-custody matters early.
5. Minimal web operator queue if HITL UX is the demo centerpiece.

---

## Files to read first

| File | Why |
| --- | --- |
| `docs/MVP.md` | This brief |
| `docs/vision.md` | Product north star |
| `docs/architecture.md` | Layering notes |
| `AGENTS.md` | Collaboration rules |
| `src/glass.mjs` | Decision + authorize + execute |
| `src/accountability.mjs` | Human/machine evidence graph |
| `scripts/demo.mjs` | Storytelling run |
