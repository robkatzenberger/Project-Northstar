# Apex audit UI

Local approve/deny review queue. Records decision receipts on disk; it does **not** execute actions.

```sh
npm run audit
```

Open http://127.0.0.1:43128

Data files live in `audit/data/`:

- `pending.jsonl` — queue rows (`PENDING` / `APPROVED` / `DENIED` / `ERROR`)
- `decisions.jsonl` — append-only decision receipts

Gate **ERROR** / **BLOCKED** assessments live in the auditor journal (`operator/gate.html` Audit trail filter), not this HITL queue. This console’s `ERROR` status is for queue-row failures only.
