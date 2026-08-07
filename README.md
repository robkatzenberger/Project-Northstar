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
  schemas/tlpx-0.1/          Shared JSON schemas (the contract)
  LICENSE
  implementations/
    javascript/              Active Node reference (CLI, tests, tech test)
    go/                      Placeholder
    java/                    Placeholder
    rust/                    Placeholder
    python/                  Placeholder
  var/                       Local audit logs (gitignored)
```

| Path | Role |
| --- | --- |
| [`docs/README.md`](docs/README.md) | Full docs hub |
| [`docs/SHARE.md`](docs/SHARE.md) | Shareable plain-language overview |
| [`docs/standard/SPEC-v0.1.md`](docs/standard/SPEC-v0.1.md) | Normative TL-PX spec |
| [`implementations/README.md`](implementations/README.md) | Multi-language guide |
| [`implementations/javascript/`](implementations/javascript/) | **Run code / tests here** |

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
