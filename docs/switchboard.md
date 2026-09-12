# Switchboard foundation

Switchboard checks whether a verified agent is whitelisted for an exact action and target. It runs before constraint screening. This first implementation is a small synchronous Node module with no dependencies, server, database, or execution capability.

## Run it

From the Northstar repository root, with Node 22 or newer:

```sh
cd apex
npm test
npm run demo:switchboard
```

No installation is needed. The demo uses the [sample registry](../config/agents.json) and generates a temporary credential in memory. It prints the permitted case and three blocked cases, without printing credentials or performing the requested actions.

## Register an agent

The trusted operator supplies a registry with an ID, positive integer version, and agent entries:

```json
{
  "id": "apex.demo",
  "version": 1,
  "agents": [
    {
      "id": "report-assistant",
      "whitelisted": true,
      "grants": [
        {
          "id": "read-demo-report",
          "action": "report.read",
          "target": "report:demo",
          "issuedBy": "operator:demo-owner"
        }
      ]
    }
  ]
}
```

Each grant binds one action to one target. Matching is exact and case-sensitive; no wildcards, path normalization, or prefix matching are performed. Empty grants grant nothing. Agent IDs and grant IDs must be unique within the registry; duplicate action-target pairs for an agent are rejected. Invalid configuration prevents initialization, including a missing or non-boolean whitelist flag. An empty registry is valid and permits nobody.

IDs, actions, and issuer names use 1–64 ASCII characters: a leading letter followed by letters, digits, `.`, `_`, `:`, or `-`. Targets use 1–256 ASCII characters, starting with a letter or digit and additionally permitting `/`. They are opaque identifiers, not resolved filesystem paths. Unsupported or extra fields are rejected rather than ignored.

`issuedBy` names the grant source supplied by the trusted operator. It is attribution from configuration, not an independently verified issuer signature. Agents must not control this registry or the host that loads it.

## Verify identity separately from the declaration

The [module](../src/switchboard.js) requires a trusted `authenticate(credential)` callback. It must synchronously return a verified principal ID or `null`. Exceptions and unsupported Promise/thenable results become blocking errors; asynchronous rejections are consumed. The request's `agentId` is only a consistency check and must match the verified identity.

The included `createTokenAuthenticator` provides a small local adapter. Each principal receives a distinct secret made from 32 cryptographically random bytes encoded as 64 lowercase hexadecimal characters. The host supplies secrets separately from the registry, for example through its environment. The adapter retains SHA-256 digests, compares fixed-size digests with `timingSafeEqual`, and never includes credentials in results. This verifies possession of a secret, not the identity or integrity of an agent binary.

```js
import { createSwitchboard, createTokenAuthenticator } from './src/switchboard.js';

// registry is parsed from operator-controlled configuration.
const authenticate = createTokenAuthenticator([
  { principalId: 'report-assistant', token: process.env.APEX_REPORT_TOKEN },
]);
const switchboard = createSwitchboard(registry, { authenticate });

// presentedCredential comes from the host's credential boundary, not agentId.
const result = switchboard.screen({
  credential: presentedCredential,
  intent: { agentId: 'report-assistant', action: 'report.read', target: 'report:demo' },
});
```

The host and authenticator are trusted embedding code. This is not an HTTP authentication service or an isolation boundary against arbitrary code in the same process. A replacement authenticator must verify credentials rather than return a caller-supplied name.

## Read the result

| Status | Meaning | Next stage |
| --- | --- | --- |
| `PASS` / `SCOPE_GRANTED` | A verified, whitelisted agent has an exact action-target grant. The result identifies the registry, checked intent, and matched grant. | `CONSTRAINT_SCREENING` |
| `DENY` | Identity is unverified, the agent is unknown/non-whitelisted, the declaration names another agent, or the scope is not granted. | None |
| `ERROR` | The request/intent is invalid or the authentication adapter failed. | None |

All results include a stable code and plain explanation. A denial cannot be overridden by a request field or sent directly to human approval. Registry, intent, and grant data returned by the module are immutable snapshots.

The accepted input is only the scope-check projection `{agentId, action, target}`. A later stage must still validate the full intended action, material arguments, constraints, and any required human verification. `PASS` is not ALLOW for the whole action, an authorization, a reusable permission ticket, or execution evidence. These diagnostic objects do not claim to be TL-PX 0.2 records.

## Change or remove access

Set `whitelisted` to `false`, remove the agent, or remove a grant in trusted configuration and create a new Switchboard instance. Recreate the authenticator when replacing credentials. The host must replace its active instances; editing the original configuration object or file does not silently change an existing instance. There is no hot reload, expiry, or automatic revocation propagation in this foundation.

The [tests](../test/switchboard.test.js) exercise real token verification, whitelist and grant checks, failure cases, immutable configuration, and credential redaction. The [builder verification record](../test/reports/switchboard-foundation-2026-09-09.md) identifies the tested bytes, results, and scope. The preserved APEX-Lite snapshot is unchanged.
