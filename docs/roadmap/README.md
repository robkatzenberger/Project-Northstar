# Roadmap & product ideas (local)

This folder captures **historical product thinking** for Northstar / Trust Layer beyond the current reference implementation.  
It is design and intent — not all of it is built yet.

**Last updated:** 2026-08-13  
**Current build baseline:** [`../BUILD-SPEC-SHEET.md`](../BUILD-SPEC-SHEET.md)  
**Disposition:** [`../reviews/build-plan-review-disposition-2026-08-13.md`](../reviews/build-plan-review-disposition-2026-08-13.md)

`priorities.md` and `language-strategy.md` are superseded as current direction. The Phase B `AUTHORIZED` snapshot token is abandoned. Read the build sheet before implementing anything from this folder.

---

## Documents in this folder

| File | Contents |
| --- | --- |
| [hm-mm-runtime.md](./hm-mm-runtime.md) | Thoughts on human↔machine and machine↔machine runtime fit |
| [priorities.md](./priorities.md) | **Superseded** August 7 sequence (PEP → snapshot token → Go) |
| [phase-a-pep.md](./phase-a-pep.md) | A16 problem retained; sequence/acceptance superseded |
| [phase-b-authz-tokens.md](./phase-b-authz-tokens.md) | **Abandoned** AUTHORIZED snapshot token |
| [phase-c-mm-handoff.md](./phase-c-mm-handoff.md) | Handoff goal retained; must not use the abandoned token |
| [openai-hf-incident-notes.md](./openai-hf-incident-notes.md) | How TL-PX relates to containment-break incidents |
| [enterprise-switchboard.md](./enterprise-switchboard.md) | When Switchboard becomes a dedicated service |
| [language-strategy.md](./language-strategy.md) | Multi-language monorepo and language choices |
| [deferred.md](./deferred.md) | Explicitly not-now items |

---

## One-line product framing

> A pre-execution trust control plane for humans and agents — identity and policy before action, with an audit trail that names both.

---

## Status snapshot (when this folder was written)

**Built and tested (JS reference):**

- Prism-compatible intent + Glass/TL-PX gate  
- Switchboard-first DENY (whitelist, credibility 0–0.99, approval routes)  
- Air-gapped mandatory audit  
- Hash-chain + HMAC seal; `glass verify`  
- Fail-closed `executeAuthorized`  
- Safe policy expression parser (no `new Function`)  
- Operator route + allowlist  
- Conformance + tech test #1 PASS; red team 16 PASS / 0 FAIL / 4 WARN  

**Also in repo:**

- Go: Switchboard, sealed audit, ops CLI, HTTP `tlpxd`  
- Java: policy + Switchboard evaluate  
- Full docs suite + shareable `docs/SHARE.md`  

**Not built yet (historical list from 2026-08-07):**

- Forced PEP / OS-enforced mediation (now destination slice 3.9, not the live slice)  
- Portable claim ticket with transactional consumption (the old AUTHORIZED snapshot token is abandoned)  
- Multi-agent handoff protocol  
- Crypto operator identity / SSO  
- Full Java audit parity  

---

## Related docs (outside this folder)

- [../BUILD-SPEC-SHEET.md](../BUILD-SPEC-SHEET.md) — current hardened build baseline  
- [../reviews/build-plan-review-disposition-2026-08-13.md](../reviews/build-plan-review-disposition-2026-08-13.md) — implementation boundary  
- [../SHARE.md](../SHARE.md) — plain-language overview for sharing  
- [../README.md](../README.md) — docs hub  
- [../security.md](../security.md) — threat model  
- [../http-api.md](../http-api.md) — Go control plane API  
- [../standard/SPEC-v0.1.md](../standard/SPEC-v0.1.md) — normative minimum standard  
