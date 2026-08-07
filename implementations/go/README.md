# Go implementation

**Status:** Policy + Switchboard + **sealed audit** + operator/execute + **HTTP control plane**  
**Module:** `github.com/Trust-Layer-AI/Project-Northstar/implementations/go`

## Features

| Feature | Status |
| --- | --- |
| Policy evaluate (JS YAML subset) | Yes |
| Switchboard-first DENY | Yes |
| Sealed audit (`prev_hash` / `audit_hash` / HMAC seal) | Yes |
| Operator approve/reject + route/allowlist | Yes |
| Chain-verified execute | Yes |
| HTTP API `tlpxd` | Yes (localhost) |

## Commands

```bash
cd implementations/go

go test ./...

# CLI evaluate with Switchboard + sealed log
go run ./cmd/tlpx evaluate \
  ../javascript/examples/intent-pii-email.json \
  ../javascript/config/policy.yaml \
  --switchboard ../javascript/config/switchboard.json \
  --log ../../var/go-audit.jsonl

go run ./cmd/tlpx verify --log ../../var/go-audit.jsonl

# HTTP control plane
go run ./cmd/tlpxd \
  --addr 127.0.0.1:8787 \
  --log ../../var/http-audit.jsonl \
  --policy ../javascript/config/policy.yaml \
  --switchboard ../javascript/config/switchboard.json
```

See [HTTP API docs](../../docs/http-api.md).

## Layout

```text
cmd/tlpx/          CLI
cmd/tlpxd/         HTTP server
internal/policy/
internal/switchboard/
internal/gate/
internal/audit/
internal/ops/
```
