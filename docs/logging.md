# Audit log locations

Trust Layer audit is **local JSONL only** (air-gapped). No cloud export by default.

## Canonical technical test path

```text
~/projects/northstar/var/tech-test-audit.jsonl
```

Resolve this path relative to your own checkout.

Audit logs live at the **monorepo root** `var/` (shared across language ports).  
JavaScript code lives under `implementations/javascript/`.

The `var/` directory is gitignored. Create or clear the log with:

```bash
cd ~/projects/northstar
mkdir -p var
: > var/tech-test-audit.jsonl
chmod 600 var/tech-test-audit.jsonl   # optional: owner read/write only
```

## CLI

From `implementations/javascript/`, always pass `--log`:

```bash
node bin/glass.mjs evaluate <intent.json> config/policy.yaml \
  --log ../../var/tech-test-audit.jsonl \
  [--switchboard config/switchboard.json]
```

## Library

When running from `implementations/javascript/`:

```js
evaluateIntent(intent, policy, {
  auditPath: "../../var/tech-test-audit.jsonl",
  switchboard
});
```
