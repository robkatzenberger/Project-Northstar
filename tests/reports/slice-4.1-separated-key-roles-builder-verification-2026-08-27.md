# Slice 4.1 separated key roles — builder verification

**Date:** 2026-08-27  
**Builder:** Codex  
**Implementation commit:** `980327d76a2b9e157bb52daeef9bf3a05e3e0d34`  
**Exact verified history:** `efa7f0f71357e922c7dc65b2a379cbb11d04ff3a` (adds a test-fixture isolation correction found by the first exact-commit rerun)  
**Disposition:** full exact-commit builder and adversarial matrix passed; independent review and acceptance deferred until Phase 4 is complete

## Scope exercised

- A purpose-bound key registry requires independent active keys for audit sealing, authorization signing, service identity, operator authentication, and tenant trust.
- Key identifiers are unique and bounded. Key material must be at least 32 bytes and cannot be reused across identifiers or roles.
- Active keys may create and verify proofs. Verify-only keys can verify historical proofs but cannot create new ones. Revoked keys can do neither.
- Role and key identifier are included in the HMAC preimage, so a proof from one purpose cannot be substituted into another role.
- Evidence rows retain their sealing key id. Reconciliation selects the exact historical audit key, allowing an old key to become verify-only while new rows use the rotated active key.
- Every issued authorization receives an authority-local authorization proof bound to its id, nonce, requester, executor, action hashes, capability, policy, adapter, tenant/environment, issuance/claim times, and execution lease.
- Claim verifies the stored authorization proof before consuming authority. Stored proof mutation or a wrong-role key id fails closed.
- Durable `SIGNING_KEY` revocation now checks configured key identity. Authorization-signing-key revocation blocks outstanding claims and later issuance; audit-key revocation makes affected evidence unverifiable and blocks new evidence creation.
- The restricted-PEP profile now requires a private five-key role bundle rather than reusing one seal secret for every role.

The authority-local authorization proof and key metadata are storage/runtime behavior. They are not private fields added to the portable `tlpx.authorization` record.

## Focused red-team results

| Attack | Result |
| --- | --- |
| Reuse identical bytes under different key ids or roles | Rejected at configuration |
| Omit a required active role | Rejected at configuration |
| Configure multiple active keys for one role | Rejected at configuration |
| Use an authorization proof as an audit proof | `KEY_ROLE_MISMATCH` |
| Mutate the proof payload or signature | Proof verification fails |
| Use a verify-only key to create a new proof | `KEY_NOT_ACTIVE` |
| Use a revoked key to create or verify a proof | `KEY_REVOKED` |
| Replace stored authorization signing key id with the audit key id | Claim returns `AUTHORIZATION_PROOF_INVALID` |
| Replace stored authorization signature | Claim returns `AUTHORIZATION_PROOF_INVALID` |
| Revoke the authorization signing key, then claim | `AUTHORIZATION_REVOKED` |
| Revoke the active authorization key, then issue | Durable fail-closed evaluation error; no authorization |
| Revoke an audit key used by history | Reconciliation returns `KEY_REVOKED` |
| Revoke an attacker-invented key id | `REVOCATION_INVALID` |
| Restart with old audit/auth keys verify-only and new keys active | Historical verification and old claim succeed; new proofs use new keys |

## Exact-commit commands and results

From `implementations/rust` at `efa7f0f71357e922c7dc65b2a379cbb11d04ff3a`:

```text
cargo fmt --all -- --check                              PASS
cargo test --all-targets --offline                     PASS (98 tests)
cargo clippy --all-targets --offline -- -D warnings    PASS
sh -n scripts/restricted-agent-acceptance.sh           PASS
```

The first exact-commit run exposed parallel test fixtures that could select the same temporary directory on clocks with coarse resolution. Commit `efa7f0f` adds a process-local atomic suffix. Three repeated focused runs and the complete matrix then passed. No product behavior was weakened to clear the failure.

From `implementations/javascript` at the same history:

```text
npm test                         PASS (935 checks across the combined suite)
npm run conformance             PASS (47/47 TL-PX 0.1)
npm run tech-test               PASS (29/29)
npm run redteam                 PASS (16 PASS, 0 FAIL, 4 documented WARN)
npm run test:rust-evidence      PASS (10/10 records)
```

The combined JavaScript suite includes 82/82 TL-PX 0.2 contract checks.

## Explicit limits and non-claims

- Key bytes are still deployment-supplied memory. Hardware-backed storage, remote KMS/HSM integration, zeroization, custody ceremonies, and automated rotation orchestration are not implemented.
- Service-identity, operator-authentication, and tenant-trust keys are purpose-reserved and misuse-tested, but the bounded local profile still authenticates peers from OS credentials and trusts locally configured policy issuers.
- Durable signing-key revocation remains authority-local state; there is still no accepted portable `tlpx.revocation` record.
- Revoking an audit key used by historical evidence deliberately makes the authority fail closed because those rows can no longer be trusted. Recovery/quarantine procedures belong to slice 4.4.
- The 3.9 separate-OS-identity acceptance script was not run. This slice does not verify 3.9, forced mediation, Section 3, or a conforming 0.2 runtime.
- Builder testing is not independent review or acceptance.

## Next gate

Build slice 4.2: durable audit export and reconciliation. Preserve historical per-key verification, ordered acknowledgement, fail-closed corruption handling, and the explicit 3.9 non-claim.
