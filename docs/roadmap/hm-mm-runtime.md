# Human↔machine and machine↔machine runtime

## Position

Northstar / TL-PX is a **strong control plane for both H-M and M-M**, not a complete agent runtime by itself.

```text
Control plane (this project):  intent → identity → policy → authorize → evidence
Runtime (host systems):        tools, networks, money movement, peer agent calls
```

The control plane only works for paths that are **mediated** through it.

---

## Human ↔ machine (core strength)

| Need | How the design covers it |
| --- | --- |
| Human declares intent | `actor_type: human` + Switchboard principal |
| Machine declares intent | Agent id + credibility |
| Human approves high risk | `REQUIRE_APPROVAL` + `approval_route` |
| Accountability | Declarer / evaluator / authorizer / executor on audit |
| Sensitive / offline envs | Air-gapped evaluate + sealed log |

**Framing:** human-in-the-loop that is *structural*, not prompt theater.

---

## Machine ↔ machine (authorization hop)

Between agents you need a **shared trust signal**, not another chat:

```text
Agent A  →  declares intent (Prism-ish)
         →  Switchboard: is A allowed? action in scope? credibility?
         →  Gate: policy for this hop
         →  optional human / multi-party escalate
         →  Agent B only acts if AUTHORIZED
         →  audit binds A's intent to B's execution
```

**Framing:** agents don’t trust each other’s vibes; they trust a shared authorization receipt (later: signed token).

Prism = signal (no judgment).  
Switchboard + gate = judgment.  
That separation is correct for multi-vendor, multi-agent systems.

---

## Design strengths

1. **External authority** — model does not self-authorize  
2. **Switchboard first** — identity/access before clever policy  
3. **Credibility 0–0.99** — progressive trust without fake certainty of 1.0  
4. **Deterministic gate** — testable, audit-friendly  
5. **Fail-closed execute + sealed audit** — enforceable, not paper policy  
6. **Dual party model** — human *and* machine on the chain  

---

## Gaps vs a full “runtime”

| Gap | Why it matters |
| --- | --- |
| Mediation optional (A16) | Bypass = no protection |
| Operator/agent crypto identity | Scale M-M needs keys / mTLS / attestation |
| Intent honesty (A12) | Declared metadata can lie |
| Multi-party / quorum | Not built |
| Portable authz token | Downstream can’t verify without shared audit |
| Cross-org federation | Standard yes; live shared registry not yet |

---

## Target architecture

```text
┌─────────────┐     ┌──────────────┐
│ Human apps  │     │ Agent runtimes│
└──────┬──────┘     └──────┬───────┘
       │ intent             │ intent
       └─────────┬──────────┘
                 ▼
        ┌─────────────────┐
        │  Switchboard    │
        │  + TL-PX gate   │
        │  + sealed audit │
        └────────┬────────┘
                 │ AUTHORIZED receipt / future signed token
       ┌─────────┴─────────┐
       ▼                   ▼
  Tools / money /       Peer agent B
  infra / email         only proceeds with valid authz
```

JS reference + Go `tlpxd` already sketch the center box.

---

## Claims discipline

| Claim | Fair? |
| --- | --- |
| Good for human–machine | **Yes — core strength** |
| Good for machine–machine as auth hop | **Yes if mediated** |
| Universal containment of rogue agents | **No** |
| Replaces sandboxes / isolation | **No** — complement, don’t replace |
| Right direction vs “model judges itself” | **Yes** |

---

## One-liners for talking about it

- **Product:** Pre-execution trust control plane for humans and agents.  
- **H-M:** Structural human-in-the-loop.  
- **M-M:** Neutral hop — shared authorization, not shared faith.  
- **Not:** “AI safety solved.”  
