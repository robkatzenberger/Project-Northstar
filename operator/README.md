# Switchboard operator UI

Local operator console for enrolling agent IDs, toggling whitelist, and editing exact action→target grants.

> Edit who may ask Switchboard for which scopes. This does not authorize execution.

## Run

From `apex/`:

```sh
npm run operator
```

Opens a loopback server at `http://127.0.0.1:43127` (no LAN bind). Registry file: `config/agents.json`.

Credentials are minted once in a modal and are **not** written to the registry.

## Scope

Operator config only. Not Glass, HITL, receipts, forced mediation, or marker PEP.
