# TL-PX 0.2 JCS golden fixtures

**Slice:** 2.2 JCS profile; slice 2.4 adds the `policy-bundle` domain
**Profile:** `northstar-jcs-v1`  
**Oracle file:** [`golden.json`](./golden.json)

Language-neutral vectors for RFC 8785 JCS under the Northstar profile, plus raw `digest_hex` and `sha256:` hash strings.

Keys sort by unsigned UTF-16 code units. Lone surrogates are rejected.

The JavaScript reference checks this file in `implementations/javascript/scripts/test-jcs.mjs`. Other languages MUST match `canonical`, `canonical_utf8_hex`, and every `sha256[domain]` exactly. They MUST reject every `reject` vector.

This is not the frozen TL-PX 0.1 conformance suite. It is not 0.1 `canonicalJson` used for audit seals.

Regenerate after a deliberate profile change:

```bash
cd implementations/javascript
node scripts/generate-jcs-fixtures.mjs
```

Then independently verify at least one vector as `SHA-256(domain_prefix || utf8(canonical))`. Domain prefixes are the byte arrays in `golden.json` `domains` (they include a trailing NUL).
