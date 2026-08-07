# Go implementation

**Status:** Placeholder — not started  

Intended use: enterprise **Trust Layer control plane** (Switchboard + gate API) as a single static binary.

## Planned shape (draft)

```text
go/
  cmd/tlpx/          CLI / server entry
  internal/gate/     evaluate, chain auth
  internal/switchboard/
  internal/audit/
  go.mod
```

## Requirements when implemented

- Conform to [TL-PX SPEC v0.1](../../docs/standard/SPEC-v0.1.md)
- Emit the same JSON record types as the JS reference
- Reuse fixtures under [`../javascript/examples/`](../javascript/examples/) where practical
- Document `go test` / `go run` here

## Why Go later

- Easy deploy (one binary)
- Natural fit for HTTP/gRPC services
- Good ops story for air-gapped enterprise installs
