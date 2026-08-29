# The Trust Layer  
### A simple overview of pre-execution trust for AI and humans

**What this is:** A plain-language introduction to the Trust Layer idea and the Northstar reference system (TL-PX / Glass / Switchboard).  

**Who it’s for:** Founders, partners, operators, engineers, and anyone who needs the *story* without reading the full technical suite.  

**Status:** The cooperative JavaScript TL-PX 0.1 reference passed technical test #1. The Rust local authority's bounded evaluate/issue/claim path is independently accepted at `aed80e2`; later Section 3 and Phase 4 candidates are not accepted. An independent Phase 4 review returned **changes requested**, and remediation is being verified locally. The separate restricted-service test using dedicated OS identities has not run. No implementation is yet an unavoidable protected-execution boundary.

---

## The problem in one sentence

**AI agents can act faster than people can supervise — and logs after the damage are too late.**

When software only *writes text*, a bad answer is embarrassing.  
When software *moves money*, *changes production*, *emails customers*, or *talks to other agents*, a bad action is an incident.

Today, a lot of “AI safety” lives **inside** the model (prompts, refusals, training). That helps. It is not enough for **real-world execution**.

---

## The idea

### Verification *before* execution

The Trust Layer is a **checkpoint between intent and action**:

1. Something (a person or an AI) **declares what it is about to do**  
2. An independent system **checks identity, rules, and risk**  
3. If needed, a **human approves**  
4. Only then may the action **run**  
5. Everything is **written down** so you can see who did what if something goes wrong  

```mermaid
flowchart LR
  A[Intent formed] --> B[Declare intent]
  B --> C[Verify]
  C --> D{Allowed?}
  D -->|Yes| E[Execute]
  D -->|Needs a human| F[Human review]
  F -->|Approve| E
  F -->|Reject| G[Stop]
  D -->|No| G
  E --> H[Audit trail]
  G --> H
```

This is the same *kind* of thinking used in other safety-critical domains: pre-flight checks, change control, admission control — **authority outside the thing proposing the action**.

---

## Why “air-gapped” matters

The Trust Layer is designed as an **external** boundary, not a setting buried inside one chatbot product.

- It can run **locally**  
- It does not require the public internet to make a decision  
- Other systems *call* it; they don’t own it  

That means it can sit in front of **many** agents and tools over time — not only one vendor’s coding assistant.

The diagram below is the **target protected-deployment shape**, not a claim that today’s Rust library already controls those systems.

```mermaid
flowchart TB
  subgraph untrusted [Things that propose actions]
    H[Human]
    A1[Agent A]
    A2[Agent B]
    S[Scripts / jobs]
  end

  subgraph trust [Trust Layer - independent]
    SW[Switchboard]
    G[Policy gate]
    AUD[(Audit log)]
  end

  subgraph world [Real systems]
    M[Money / APIs]
    P[Production]
    C[Customer tools]
  end

  H --> SW
  A1 --> SW
  A2 --> SW
  S --> SW
  SW --> G
  G --> AUD
  G -->|only if authorized| M
  G -->|only if authorized| P
  G -->|only if authorized| C
```

---

## The pieces (simple names)

| Piece | Plain English | One-liner |
| --- | --- | --- |
| **Prism** | The “I intend to…” note | Describes intent. Does **not** approve or deny. |
| **Switchboard** | The bouncer + directory | Who are you? Are you allowed in? How trusted are you? Who should approve you? |
| **Glass / TL-PX gate** | The rulebook | Given this intent + rules, **allow**, **ask a human**, or **deny**. |
| **Human approval** | The supervisor | For higher risk: a person must approve before go. |
| **Executor** | The last lock | In the target PEP, will **not** run the real action unless the gate (and audit) say authorized. |
| **Audit log** | The black box recorder | Append-only history: who declared, who decided, who approved, what ran. |

Today, the JavaScript executor is a historical cooperative 0.1 reference: it blocks mediated calls, but a process retaining direct access can bypass it. The Go HTTP implementation is also historical/cooperative and trusts caller-supplied actor names; neither is a 0.2 authority. The Rust candidate adds durable one-time claim state, a sealed evidence outbox, and one bounded direct-argv marker demo. That demo runs under the caller's identity and can also be bypassed. A restricted-service candidate exists, but its defining dedicated-identity test remains unrun. Forced mediation is not established.

### How they fit together

This is the intended end-to-end flow. Today’s JavaScript executor and Rust 3.8 runner are cooperative; neither removes alternate direct routes to a protected capability.

```mermaid
flowchart TD
  I[Declared intent] --> SW[Switchboard]
  SW -->|Not registered / not allowed| DENY1[DENY - stop]
  SW -->|OK, with trust score| POL[Policy gate]
  POL -->|Safe enough| ALLOW[ALLOW]
  POL -->|Needs review| HITL[Human approval]
  HITL -->|Approve| ALLOW
  HITL -->|Reject| DENY2[DENY - stop]
  ALLOW --> EX[Executor]
  EX -->|Authorized| RUN[Action runs]
  EX -->|Not authorized| BLOCK[Action blocked]
  DENY1 --> LOG[(Audit log)]
  DENY2 --> LOG
  RUN --> LOG
  BLOCK --> LOG
  HITL --> LOG
  POL --> LOG
  SW --> LOG
```

**Important order:**  
**Switchboard is first.** If Switchboard denies you, the detailed rulebook never gets a chance to “maybe allow” you. That is intentional.

---

## Identity and credibility (Switchboard)

Every agent or machine should be a known **principal** (like an employee badge):

- **Whitelist** — are you allowed to use this gate at all?  
- **Allowed actions** — are you allowed to do *this kind* of thing?  
- **Credibility score** — **0.00 to 0.99** (never a perfect 1.0)  
- **Approval route** — if a human is needed, *who* gets the ticket?

```mermaid
flowchart LR
  subgraph score [Credibility bands - example defaults]
    L[Low under 0.40<br/>usually needs a human]
    M[Medium 0.40–0.84<br/>normal rules]
    H[High 0.85+<br/>more trust, still not unlimited]
  end
```

**Credibility is operator-managed**, not “the AI scored itself.”  
It is a dial humans set (and can adjust after outcomes), so the system stays **predictable**.

---

## Decisions people will see

| Decision | Meaning |
| --- | --- |
| **ALLOW** | Rules say this can proceed without waiting for a person |
| **REQUIRE APPROVAL** | Pause — a human on the approval route must approve |
| **DENY** | Stop — especially access/identity failures from Switchboard |

When something tries to **run**:

| Status | Meaning |
| --- | --- |
| **AUTHORIZED** | Allowed to execute |
| **PENDING** | Waiting on a human |
| **DENIED** | Must not execute |
| **EXECUTED / BLOCKED / FAILED** | What the executor recorded after the attempt |

---

## Accountability (when something goes wrong)

The goal is not blame theater. The goal is a **clear evidence trail**:

```mermaid
flowchart LR
  D[Who declared the intent?] --> E[Who evaluated the rules?]
  E --> A[Who approved - if anyone?]
  A --> X[Who executed?]
  X --> R[What was the outcome?]
```

Parties can be **human** or **machine**.  
That matters when the failure was:

- a model that mis-declared intent,  
- a policy that was too loose,  
- a human who approved too quickly, or  
- a runtime that ran something it shouldn’t have.

The JavaScript 0.1 reference uses a local append-only **JSONL** audit file. The Rust authority uses SQLite for live state and, at `c9bdd0f`, a separate sealed evidence outbox.
Canonical technical-test log path:

```text
var/tech-test-audit.jsonl
```

---

## What this is *not*

To keep expectations honest:

| It is | It is not |
| --- | --- |
| A pre-execution checkpoint | A promise that every program on Earth is contained |
| A rule + identity system | “The AI has ethics now” |
| Evidence for incident review | Automatic legal judgment of liability |
| Designed for air-gap / local control | Dependent on one cloud vendor to decide |
| Open *minimum standard* direction (TL-PX) | The entire enterprise product surface |

If a system **never calls** the Trust Layer and runs the action anyway, the Trust Layer cannot stop it — just like a door lock cannot stop someone who walks through a hole in the wall. Real deployments must **route sensitive actions through the gate**.

---

## Open “to an extent”

The long-term shape:

```mermaid
flowchart TB
  subgraph open [Open / shareable core]
    P[Prism - intent signal]
    T[TL-PX minimum standard]
    R[Reference gate + tests]
  end

  subgraph product [Product extensions - optional]
    G[Glass enterprise features]
    OPS[Ops console, multi-tenant, signed tokens...]
  end

  P --> T
  T --> R
  T --> G
  G --> OPS
```

- **Open core:** so others can implement the same minimum and pass tests  
- **Product layer:** stronger enterprise capabilities without forking the meaning of “conforming”

---

## Where the project is today (plain status)

| Item | Status |
| --- | --- |
| Idea & architecture | Documented in the **Northstar** repository |
| JavaScript TL-PX 0.1 Switchboard and cooperative gate | Working for mediated calls |
| JavaScript audit + fail-closed cooperative executor | Working; direct bypass remains possible |
| Rust local evaluate/issue/atomic-claim authority | Independently accepted at `aed80e2`; no side effect or PEP |
| Rust bounded sealed evidence outbox | Builder-verified at `c9bdd0f`; independent acceptance pending |
| Policy provenance, precedence, and ordering contract | Local slice 2.4 commit `a87f822`; full Section 3 verification deferred; not accepted |
| Rust policy content binding and active-manifest selection | Local slice 3.1 commit `1addb5c`; full Section 3 verification deferred; not accepted |
| Rust bounded cooperative command runner | Local slice 3.8 commit `7c41450`; full exact-commit builder matrix passed; bypassable and not independently accepted |
| Automated tests + 0.1 and schema-only 0.2 conformance | Passing |
| Formal technical test #1 | **Passed (29/29)** for the JavaScript cooperative reference |
| Forced mediation / protected-execution PEP | Not implemented |
| GitHub for *this* repo | A baseline exists remotely; current post-baseline commits and working-tree changes remain local until explicitly pushed |
| Related public pieces | Prism / APEX-Lite / Trust Layer site under Trust-Layer-AI |

---

## A day-in-the-life story

**Scenario:** A support agent wants to email a customer with an attachment that may include personal data.

This is an **intended protected-deployment story**. The current Rust candidate does not send email or provide the unavoidable executor shown here; its 3.8 command demo is deliberately cooperative.

```mermaid
sequenceDiagram
  participant Agent as Support agent AI
  participant SB as Switchboard
  participant Gate as Policy gate
  participant Human as Ops human
  participant Mail as Email system
  participant Log as Audit log

  Agent->>SB: I intend to send_email to external_user, PII
  SB->>SB: Known? Whitelisted? Action allowed? Credibility?
  SB->>Gate: OK to evaluate
  Gate->>Gate: Rule: email + PII needs human
  Gate->>Log: Decision REQUIRE_APPROVAL
  Gate-->>Human: Approval request route: ops.alex
  Note over Agent,Mail: Action cannot run yet
  Human->>Gate: APPROVE
  Gate->>Log: Operator approved
  Gate->>Mail: Executor runs only now
  Mail->>Log: EXECUTED
```

If the agent was **unknown** or **not whitelisted**, Switchboard would **DENY** immediately — no email, no “please approve this stranger.”

---

## Why this matters now

AI is moving from **answers** to **actions**.  
Organizations will need a shared way to say:

> “Nothing important happens until intent is declared, checked, and — when required — approved — and we can prove that trail later.”

The Trust Layer is that pattern: **simple enough to explain, strict enough to test, open enough to become a minimum standard.**

---

## If you want more depth

| Audience | Document |
| --- | --- |
| Full docs hub | [docs/README.md](./README.md) |
| Hands-on | [getting-started.md](./getting-started.md) |
| Spec (normative) | [standard/SPEC-v0.1.md](./standard/SPEC-v0.1.md) |
| Security honesty | [security.md](./security.md) |

---

## One closing line

**Declare intent. Check who and what. Approve when it matters. Execute only when authorized. Leave a trail.**

That’s the Trust Layer.

---

*Document version: shareable overview · Northstar / TL-PX · local reference · not legal advice · not a patent claim set*
