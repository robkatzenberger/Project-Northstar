# Northstar — Trust Layer (multi-language monorepo)

**Codename:** Northstar  
**What this is:** Trust Layer pre-execution checkpoint — **TL-PX** minimum standard, docs, schemas, and language implementations.

> Intent before action. Switchboard first. Gate second. Execute only if authorized.

**License:** Apache-2.0  
**Primary implementation today:** JavaScript (Node 18+)

---

## Repository layout

```text
northstar/
  docs/                      Language-agnostic documentation
  docs/roadmap/              Product ideas & phased plan (H-M/M-M, PEP, tokens)
  schemas/tlpx-0.1/          Shared JSON schemas (the contract)
  LICENSE
  implementations/
    javascript/              Full Node reference
    go/                      Switchboard + sealed audit + HTTP control plane
    java/                    Policy + Switchboard evaluate
    rust/ python/            Placeholders
  var/                       Local audit logs (gitignored)
```

| Path | Role |
| --- | --- |
| [`docs/README.md`](docs/README.md) | Full docs hub |
| [`NORTHSTAR-SESSION-START.md`](NORTHSTAR-SESSION-START.md) | Fresh-context instructions and required reading order |
| [`docs/BUILD-SPEC-SHEET.md`](docs/BUILD-SPEC-SHEET.md) | Proposed hardened build baseline targeting TL-PX 0.2 |
| [`docs/SHARE.md`](docs/SHARE.md) | Shareable plain-language overview |
| [`docs/roadmap/`](docs/roadmap/README.md) | **Ideas & next phases** |
| [`docs/standard/SPEC-v0.1.md`](docs/standard/SPEC-v0.1.md) | Normative TL-PX spec |
| [`implementations/README.md`](implementations/README.md) | Multi-language guide |
| [`implementations/javascript/`](implementations/javascript/) | **Run JS code / tests here** |

---

## Quick start (JavaScript reference)

```bash
cd implementations/javascript

npm test
npm run conformance
npm run tech-test

node bin/glass.mjs evaluate examples/intent-safe.json config/policy.yaml \
  --log ../../var/tech-test-audit.jsonl
```

Canonical audit log (monorepo root):

```text
var/tech-test-audit.jsonl
```

---

## Adding another language

1. Use the placeholder under `implementations/<lang>/` (or create one).  
2. Implement against [`docs/standard/SPEC-v0.1.md`](docs/standard/SPEC-v0.1.md) and `schemas/`.  
3. Keep records interoperable with the JS reference.  
4. Update [`implementations/README.md`](implementations/README.md).

You do **not** need every language for the open standard — one solid reference + schemas is enough. Extra languages are ports for specific environments (JVM, Go services, Rust PEPs, etc.).

## Hardened-core direction

TL-PX 0.1 and the current JavaScript implementation remain the verified historical reference. The proposed hardened build targets a separately versioned TL-PX 0.2 contract rather than silently rewriting v0.1.

The accepted direction is a small Rust authoritative core/PEP, a TypeScript-readable reference and conformance oracle, and other languages as SDKs/adapters. The foundational rule is:

> One authorization permits one authenticated executor to perform one exact action, one time.

See [`docs/BUILD-SPEC-SHEET.md`](docs/BUILD-SPEC-SHEET.md) for planned requirements. Planned behavior is not current implementation status.

---

## Status

- [x] TL-PX 0.1 + JS reference + air-gap hardening  
- [x] Technical test #1 PASS (JS)  
- [x] Multi-language folder layout  
- [x] Audit hash-chain + HMAC seal (`glass verify`)  
- [x] Go: Switchboard + sealed audit + CLI + HTTP `tlpxd`  
- [x] Java: Switchboard + evaluate  
- [ ] Rust / Python ports  
- [ ] Java sealed audit / full ops parity  
- [ ] Production HA multi-tenant control plane

---

## Related

- [Trust Layer AI site](https://trust-layer-ai.github.io/Trust-Layer-AI/)  
- [Prism](https://github.com/Trust-Layer-AI/prism-protocol)  
- [Trust-Engine](https://github.com/Trust-Layer-AI/Trust-Engine)  
