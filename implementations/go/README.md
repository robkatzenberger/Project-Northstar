# Go implementation (skeleton)

**Status:** Early skeleton — policy evaluate only  
**Module:** `github.com/Trust-Layer-AI/Project-Northstar/implementations/go`

## What works

- Load the shared JS reference `policy.yaml` (flat rule format)
- Evaluate intents → `ALLOW` | `REQUIRE_APPROVAL` with TL-PX-shaped decision JSON
- Unit tests against `../javascript/examples` policy rules

## Not yet

- Switchboard
- Sealed audit JSONL
- Operator resolve / executeAuthorized
- HTTP service

## Commands

```bash
cd implementations/go

go test ./...

# Evaluate a JS fixture (from this directory)
go run ./cmd/tlpx evaluate \
  ../javascript/examples/intent-safe.json \
  ../javascript/config/policy.yaml

go run ./cmd/tlpx evaluate \
  ../javascript/examples/intent-pii-email.json \
  ../javascript/config/policy.yaml
```

## Layout

```text
go/
  go.mod
  cmd/tlpx/main.go
  internal/policy/
  internal/gate/
  README.md
```

Target direction: enterprise control-plane service sharing `schemas/tlpx-0.1/` contracts.
