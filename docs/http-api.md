# HTTP control plane (`tlpxd`)

> **Status:** Historical/cooperative TL-PX 0.1-era Go control plane. It trusts caller-supplied identities and is not the Rust authority, a TL-PX 0.2 adapter endpoint, a protected-execution PEP, or a network-egress PEP. `/v1/execution` records a caller report; it does not execute or mediate outbound HTTP.

Local air-gapped **Go** service wrapping evaluate / operator / caller-reported execution / verify.

## Run

```bash
cd implementations/go

# requires Go toolchain
go run ./cmd/tlpxd \
  --addr 127.0.0.1:8787 \
  --log ../../var/http-audit.jsonl \
  --policy ../javascript/config/policy.yaml \
  --switchboard ../javascript/config/switchboard.json
```

Binds **localhost by default** (not public internet).

## Endpoints

| Method | Path | Body / notes |
| --- | --- | --- |
| GET | `/healthz` | liveness |
| GET | `/v1/verify` | audit integrity |
| GET | `/v1/auth/{receipt_id}` | chain-derived authorization |
| POST | `/v1/evaluate` | `{ "intent": { ... } }` optional `policy_path`, `switchboard_path` |
| POST | `/v1/operator-action` | `{ receipt_id, operator_id, outcome, note? }` |
| POST | `/v1/execution` | `{ receipt_id, executor_id, status, summary? }` |

### Example

```bash
curl -s http://127.0.0.1:8787/healthz

curl -s -X POST http://127.0.0.1:8787/v1/evaluate \
  -H 'content-type: application/json' \
  -d '{
    "intent": {
      "agent": "agent.docs.summarizer",
      "actor_type": "machine",
      "intent_summary": "Summarize report",
      "action": "summarize_report",
      "risk": "low",
      "data_classes": []
    }
  }'
```

Sealed audit is written to `--log` (same hash-chain + HMAC as JS).

## CLI counterpart

```bash
go run ./cmd/tlpx evaluate intent.json policy.yaml --switchboard sb.json --log ../../var/go-audit.jsonl
go run ./cmd/tlpx verify --log ../../var/go-audit.jsonl
```
