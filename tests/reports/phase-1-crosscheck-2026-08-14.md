# Phase 1 / slice 2.1 crosscheck follow-up — 2026-08-14

**Prior implementation commit:** `31175c5` (`fix(js): fail-closed policy compile for slices 1.1-1.2`)  
**Prior contract commit:** `593439b` (`docs: freeze TL-PX 0.1 and draft 0.2 decision contract`)  
**This follow-up:** local commit after independent combined crosscheck (not pushed)  
**Does not overwrite:** `phase-1-policy-compile-2026-08-14.md` (point-in-time; its “no commit” line is inaccurate — that report is contained in `31175c5`)

## Runtime

| Tool | Version |
| --- | --- |
| Node | v25.5.0 |
| npm | 11.8.0 |

Commands run in `implementations/javascript/`.

## Findings closed

1. **High — forgeable compiled marker.** `POLICY_COMPILED` is no longer exported. `compilePolicy` tracks packs it actually produced in a module-private `WeakSet`. A caller-stamped empty pack no longer returns `ALLOW`.
2. **Medium — invalid `policy_pack_id`.** A supplied non-string or empty id fails compile. Absence still defaults to `"default"`. Duplicate YAML `policy_pack_id` fails parse.
3. **Medium — claim ticket / claim record / sealing language** in `SPEC-v0.2.md`: adapter binding and atomic online claim restated; dedicated `tlpx.authorization_claim`; sealing described as an evidence-chain property, not a JSON field.

## Commands

| Command | Exit | Result |
|---|---:|---|
| `npm test` | 0 | 165 assertions: 25 core, 24 Switchboard, 19 air-gap, 11 policy/operator, 86 compile/negative |
| `npm run conformance` | 0 | 47 passed, 0 failed |

Red team and technical test were not re-run in this follow-up; they remain the `31175c5` results (16 PASS / 0 FAIL / 4 WARN, and 29/29) and are not affected by the marker/`policy_pack_id` compile path except through the same `compilePolicy` gate.

## Still true

- TL-PX 0.1 remains frozen rather than patched.
- No Rust, token, PEP, or 0.2 runtime records.
- A process that never calls the gate can still act (A16).
