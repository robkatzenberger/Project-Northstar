# CLI Reference

Binary: `bin/glass.mjs` (also named `tlpx` in `package.json`).

```bash
node bin/glass.mjs <command> [args] [flags]
node bin/glass.mjs help
```

**Air-gapped:** durable commands require `--log <path>` to an append-only JSONL audit file.

**Recommended log for formal work:**

```bash
--log var/tech-test-audit.jsonl
```

---

## Global flags

| Flag | Description |
| --- | --- |
| `--log PATH` | Audit JSONL path (required for evaluate / approve / reject / execute / auth / chain / incident) |
| `--switchboard PATH` | Switchboard config (default: `config/switchboard.json` if present) |
| `--no-switchboard` | Disable Switchboard for this evaluate |
| `--operator ID` | Human operator id (approve/reject) |
| `--executor ID` | Executor id (execute) |
| `--status STATUS` | `EXECUTED` \| `BLOCKED` \| `FAILED` |
| `--note TEXT` | Optional operator note |
| `--summary TEXT` | Optional execution summary |
| `--why TEXT` | Incident description |
| `--observed ACTION` | Observed action for incident mismatch analysis |
| `--severity LEVEL` | Incident severity |

---

## Commands

### `evaluate`

```bash
node bin/glass.mjs evaluate <intent.json> <policy.yaml> --log PATH [--switchboard PATH] [--no-switchboard]
```

Reads intent (example style, Prism signal, or flat APEX-style JSON), evaluates with optional Switchboard, appends decision, prints JSON.

**Intent file shapes accepted**

1. Example style: `agent`, `intent_summary`, optional `action` / `risk` / …  
2. Full Prism signal with `prism_id` + `prism_version`  
3. Flat: `actor`, `action`, `risk`, `data_classes`, …

### `switchboard`

```bash
node bin/glass.mjs switchboard <agent_id> [--switchboard PATH]
```

Lookup principal + thresholds. Does not write audit.

### `approve` / `reject`

```bash
node bin/glass.mjs approve <receipt_id> --operator ID --log PATH [--note TEXT]
node bin/glass.mjs reject  <receipt_id> --operator ID --log PATH [--note TEXT]
```

Single terminal outcome per receipt. Second call fails.

### `execute`

```bash
node bin/glass.mjs execute <receipt_id> \
  --executor ID \
  --status EXECUTED|BLOCKED|FAILED \
  --log PATH \
  [--summary TEXT]
```

**Chain-verified:** authorization read only from audit.  
`EXECUTED` fails unless audit says `AUTHORIZED`.

### `auth`

```bash
node bin/glass.mjs auth <receipt_id> --log PATH
```

Prints `resolveAuthorizationFromAudit` result (status, source, terminal).

### `chain`

```bash
node bin/glass.mjs chain <receipt_id> --log PATH
```

Prints ordered records for the receipt.

### `incident`

```bash
node bin/glass.mjs incident <receipt_id> --log PATH --why "what went wrong" \
  [--observed action] [--severity high]
```

Builds accountability report; persists incident + report to audit when implemented path uses persist.

### `help`

```bash
node bin/glass.mjs help
```

---

## End-to-end example

```bash
cd ~/projects/northstar
LOG=var/tech-test-audit.jsonl
: > "$LOG"

# 1) Evaluate (may ALLOW / REQUIRE_APPROVAL / DENY)
node bin/glass.mjs evaluate examples/intent-pii-email.json config/policy.yaml --log "$LOG"

# 2) If pending — approve (use approval_route from decision)
node bin/glass.mjs approve <receipt_id> --operator human.ops.alex --log "$LOG"

# 3) Confirm
node bin/glass.mjs auth <receipt_id> --log "$LOG"

# 4) Execute record
node bin/glass.mjs execute <receipt_id> --executor runtime.mailer --status EXECUTED --log "$LOG"

# 5) Inspect
node bin/glass.mjs chain <receipt_id> --log "$LOG"
```

---

## Exit behavior

- Successful JSON printed to stdout  
- Errors on stderr; process exits non-zero  
- Prefer checking `decision` / `authorization_status` in JSON for automation logic  

---

## npm scripts (wrappers)

| Script | Command |
| --- | --- |
| `npm test` | unit + switchboard + air-gap |
| `npm run conformance` | TL-PX conformance |
| `npm run demo` | narrative demo |
| `npm run demo:switchboard` | Switchboard demo |
| `node scripts/tech-test.mjs` | formal technical test #1 |
| `node scripts/adversarial-redteam.mjs` | red team |

See [Testing](./testing.md).
