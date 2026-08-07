# Python implementation

**Status:** Placeholder — not started  

Intended use: adapters in data/ML stacks, notebooks, or Python agent frameworks.

## Planned shape (draft)

```text
python/
  pyproject.toml
  src/tlpx/
  tests/
```

## Requirements when implemented

- Conform to [TL-PX SPEC v0.1](../../docs/standard/SPEC-v0.1.md)
- Same JSON records; pytest fixtures aligned with JS examples

## Note

Prefer a thin client that calls a Go/JS gate service if Python is only the agent side. Full in-process port is optional.
