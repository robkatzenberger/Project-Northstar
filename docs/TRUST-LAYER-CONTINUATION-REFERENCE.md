# Trust Layer source repositories — continuation reference

**Reviewed:** 2026-09-04
**Role:** Design-lineage and product-positioning reference; not normative Northstar authority or acceptance evidence

## Local reference checkouts

The repositories below are cloned under the ignored local `reference-repos/` directory. Exact reviewed commits are pinned here so the review remains reproducible even after those repositories change.

| Repository | Reviewed commit | Reference role |
| --- | --- | --- |
| [Trust Layer AI](https://github.com/Trust-Layer-AI/Trust-Layer-AI) | `daf4ab51bcd45682566d237253e9e49f49c7a999` | Public narrative and ecosystem framing |
| [Prism Protocol](https://github.com/Trust-Layer-AI/prism-protocol) | `08724a16a2740572189796d84ee20e9e03f0c853` | Historical metadata-only intent signal |
| [Trust Engine / APEX-Lite](https://github.com/Trust-Layer-AI/Trust-Engine) | `b0dadd6d0f9c08f7c74e8524d324afd4623610aa` | Historical cooperative deterministic gate |

Local paths:

- `reference-repos/Trust-Layer-AI/`
- `reference-repos/prism-protocol/`
- `reference-repos/Trust-Engine/`

These are nested Git checkouts for inspection only. They are not submodules, vendored dependencies, or part of Northstar's conformance line.

## What Northstar should retain

The earlier repositories establish several durable ideas that remain aligned with Northstar:

- Place a visible boundary between proposed intent and real-world action.
- Keep intent metadata outside hidden model reasoning and chain-of-thought.
- Make the protocol model- and framework-agnostic.
- Keep the enforcing core small, deterministic, inspectable, and operator-controlled.
- Treat human escalation as a normal collaboration path rather than as agent failure.
- Use plain examples and a small public reference implementation to make the concept understandable.
- Preserve the product invariant that execution must not exceed authorization.

These are design and communication principles. They do not, by themselves, establish authenticated identity, authorization, execution, or evidence integrity.

## Prism boundary

Prism v0.1 is a five-field, self-declared metadata envelope. Its `agent`, `timestamp`, identifier, and free-text summary are not authenticated authority inputs. Prism explicitly does not judge, verify, or enforce an action.

Northstar may accept a Prism signal as optional descriptive ingress metadata, but it must not use that signal as proof of principal identity, trusted time, policy scope, action completeness, authorization, or execution. Any action entering the TL-PX 0.2 path must be transformed into and validated as a complete Submitted Intent, then bound to authenticated context and authority-derived policy data.

Prism should remain metadata-only. Do not expand it into a parallel policy or authorization protocol.

## APEX-Lite boundary

APEX-Lite is valuable as a readable historical demonstration of an external deterministic gate. It is not a hardened authority or PEP. At the reviewed commit it deliberately or implicitly has the following properties:

- decisions are `ALLOW` or `REQUIRE_APPROVAL`; even a rule written with `deny: true` becomes `REQUIRE_APPROVAL`;
- the default is `ALLOW` when no rule matches;
- receipts declare `blocking: false` and do not mediate execution;
- actor, risk, target, timestamps, and operator names are caller-supplied strings;
- policy expressions are interpreted during evaluation rather than fully compiled before any request, so an invalid later rule can remain undiscovered after an earlier return;
- approval records do not authenticate the operator or bind a canonical exact action;
- receipt identifiers are readable derivatives of intent ID and time rather than authority-generated collision-resistant identifiers;
- the JSONL audit is append-only by convention but is not transactionally coupled, hash-chained, sealed, or concurrency-safe;
- there is no short-lived authorization, authenticated executor binding, atomic single-use claim, revocation transaction, execution receipt, unknown-outcome reconciliation, or forced mediation.

Northstar must preserve APEX-Lite as historical cooperative evidence and must not inherit these behaviors into TL-PX 0.2.

## Public narrative boundary

The Trust Layer site is useful for its concise problem statement, diagrams, and separation of Prism from enforcement. Some wording is aspirational relative to the reviewed public implementations, including claims around signed authorization tokens, tamper-evident audit, identity verification, and verifiable execution. The site's Prism example also uses `acknowledgment` where the Prism v0.1 schema requires `prism_version`.

Before public reuse, reconcile each product claim with the exact accepted Northstar commit and evidence. Until the corresponding enforcement path is independently accepted, prefer scoped phrases such as:

- "pre-execution authorization boundary";
- "experimental enforcement boundary";
- "builder-verified candidate";
- "authorization and execution evidence are distinct";
- "forced mediation remains unverified".

Do not use the earlier site's broad language as implementation evidence.

## Continuation decision

Use the three repositories to preserve lineage and clarify the public story, while keeping the current Northstar contract and Rust authority as the only hardened continuation path.

The next continuation gate should remain narrow:

1. Freeze feature expansion.
2. Review the runtime remediation at `77d77b8` for post-lock production time, administrator cleanup, and exact evidence validation.
3. With owner approval, produce a clean exact candidate commit and rerun the complete matrix plus the administrator gate. The last committed marker evidence is source `f025332`, evidence commit `32c049e`, report `064717`; it is not evidence for later changes.
4. Obtain independent re-review of the new exact candidate.
5. Only then consider a future restricted-egress profile or another real external capability for which exact-action binding, authenticated single-use claim, and durable execution evidence close a gap not covered by ordinary sandboxing.

If that sequence cannot establish a meaningful enforcement advantage, preserve Prism and APEX-Lite as historical references and stop expanding Northstar. If it succeeds, continue Northstar as a provider-neutral authorization-and-receipt kernel rather than a universal trust, semantic-consequence predictor, containment, or orchestration platform.

## Authority order

When these sources disagree, use this order:

1. accepted TL-PX 0.2 specification and schemas;
2. exact-commit independent Northstar acceptance evidence;
3. current Northstar build specification and explicitly scoped candidates;
4. APEX-Lite and Prism as historical/design references;
5. Trust Layer public-site language as product narrative only.
