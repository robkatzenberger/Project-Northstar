# Go implementation (skeleton)

**Status:** Policy evaluate + **Switchboard-first** routing  
**Module:** `github.com/Trust-Layer-AI/Project-Northstar/implementations/go`

## What works

- Load shared JS `policy.yaml` + `switchboard.json`
- Switchboard hard DENY (unknown / not whitelisted / action denied)
- Credibility flags (`low_credibility` / `high_trust`) for policy
- TL-PX-shaped decision JSON (`ALLOW` | `REQUIRE_APPROVAL` | `DENY`)
- Unit tests

## Not yet

- Sealed audit JSONL
- Operator resolve / executeAuthorized
- HTTP service

## Commands

```bash
cd implementations/go

go test ./...

# Policy only
go run ./cmd/tlpx evaluate \
  ../javascript/examples/intent-safe.json \
  ../javascript/config/policy.yaml

# With Switchboard
go run ./cmd/tlpx evaluate \
  ../javascript/examples/intent-unknown-agent.json \
  ../javascript/config/policy.yaml \
  ../javascript/config/switchboard.json

go run ./cmd/tlpx evaluate \
  ../javascript/examples/intent-pii-email.json \
  ../javascript/config/policy.yaml \
  ../javascript/config/switchboard.json
```

## Layout

```text
go/
  cmd/tlpx/
  internal/policy/
  internal/switchboard/
  internal/gate/
  go.mod
```
