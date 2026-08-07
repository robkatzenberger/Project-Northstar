# CLI Reference

Binary: `implementations/javascript/bin/glass.mjs` (npm name `tlpx` / `glass`).

```bash
cd ~/projects/northstar/implementations/javascript

node bin/glass.mjs <command> [args] [flags]
node bin/glass.mjs help
```

**Air-gapped:** durable commands require `--log <path>` to an append-only JSONL audit file.  
Lines are **hash-chained + HMAC-sealed** (seal key beside the log as `.<basename>.seal`, or `TLPX_AUDIT_SEAL`).

**Recommended log (monorepo root):**

```bash
--log ../../var/tech-test-audit.jsonl
```

---

## Global flags

| Flag | Description |
| --- | --- |
| `--log PATH` | Audit JSONL path (required for evaluate / approve / reject / execute / auth / verify / chain / incident) |
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

### `switchboard`

```bash
node bin/glass.mjs switchboard <agent_id> [--switchboard PATH]
```

### `approve` / `reject`

```bash
node bin/glass.mjs approve <receipt_id> --operator ID --log PATH [--note TEXT]
node bin/glass.mjs reject  <receipt_id> --operator ID --log PATH [--note TEXT]
```

### `execute`

```bash
node bin/glass.mjs execute <receipt_id> \
  --executor ID \
  --status EXECUTED|BLOCKED|FAILED \
  --log PATH \
  [--summary TEXT]
```

### `auth`

```bash
node bin/glass.mjs auth <receipt_id> --log PATH
```

### `verify`

```bash
node bin/glass.mjs verify --log PATH
```

Verifies hash-chain + HMAC seals from genesis. Exit `2` if integrity fails.

### `chain` / `incident`

```bash
node bin/glass.mjs chain <receipt_id> --log PATH
node bin/glass.mjs incident <receipt_id> --log PATH --why "what went wrong" \
  [--observed action] [--severity high]
```

---

## End-to-end example

```bash
cd ~/projects/northstar/implementations/javascript
LOG=../../var/tech-test-audit.jsonl
: > "$LOG"
rm -f ../../var/.tech-test-audit.jsonl.seal   # optional: fresh seal key

node bin/glass.mjs evaluate examples/intent-pii-email.json config/policy.yaml --log "$LOG"
node bin/glass.mjs approve <receipt_id> --operator human.ops.alex --log "$LOG"
node bin/glass.mjs auth <receipt_id> --log "$LOG"
node bin/glass.mjs verify --log "$LOG"
node bin/glass.mjs execute <receipt_id> --executor runtime.mailer --status EXECUTED --log "$LOG"
node bin/glass.mjs chain <receipt_id> --log "$LOG"
```

---

## npm scripts

Run from `implementations/javascript/`:

| Script | Command |
| --- | --- |
| `npm test` | unit + switchboard + air-gap |
| `npm run conformance` | TL-PX conformance |
| `npm run demo` | narrative demo |
| `npm run demo:switchboard` | Switchboard demo |
| `npm run tech-test` | formal technical test #1 |
| `npm run redteam` | adversarial suite |

See [Testing](./testing.md).
