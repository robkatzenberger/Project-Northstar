# Audit log locations

Trust Layer audit is **local JSONL only** (air-gapped). No cloud export by default.

## Canonical technical test path

```text
~/projects/northstar/var/tech-test-audit.jsonl
```

Absolute:

```text
/Users/home/projects/northstar/var/tech-test-audit.jsonl
```

The `var/` directory is gitignored. Create or clear the log with:

```bash
mkdir -p var
: > var/tech-test-audit.jsonl
chmod 600 var/tech-test-audit.jsonl   # optional: owner read/write only
```

## CLI

Always pass `--log`:

```bash
node bin/glass.mjs evaluate <intent.json> config/policy.yaml \
  --log var/tech-test-audit.jsonl \
  [--switchboard config/switchboard.json]
```

## Library

```js
evaluateIntent(intent, policy, {
  auditPath: "/Users/home/projects/northstar/var/tech-test-audit.jsonl",
  switchboard
});
```
