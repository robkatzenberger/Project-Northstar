# Language strategy

## Clarification

Current primary implementation is **JavaScript (Node ES modules)**, not Java.

---

## Fit by language

| Language | Role |
| --- | --- |
| **JavaScript / TypeScript** | Best for open reference, iteration, adapters, hooks |
| **Go** | Best near-term for **control plane service** (`tlpxd`), single binary, ops-friendly |
| **Java / Kotlin** | Optional for JVM-heavy enterprises; not required for “enterprise” |
| **Rust** | Later: high-assurance PEP sidecar |
| **Python** | Thin client / agent-side adapters; full port optional |

---

## Monorepo layout (current)

```text
northstar/
  docs/                 # language-agnostic
  schemas/tlpx-0.1/     # shared contract
  implementations/
    javascript/         # full reference
    go/                 # Switchboard + sealed audit + HTTP
    java/               # policy + Switchboard evaluate
    rust/               # placeholder
    python/             # placeholder
  var/                  # local audits
```

**Rule:** ports implement [SPEC-v0.1](../standard/SPEC-v0.1.md) and shared schemas; they don’t redefine Prism casually.

---

## Recommendation

1. Keep **JS** as the conformance / open-core reference  
2. Grow **Go** as the enterprise-shaped service + PEP backend  
3. Keep **Java** as JVM evaluate path; add audit only if a buyer needs in-process JVM  
4. Don’t start Rust/Python until PEP + tokens need a specialized host  

Java is a **valid enterprise option**, not the default best language for Trust Layer.
