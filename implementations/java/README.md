# Java implementation

**Status:** Policy evaluate + **Switchboard-first** routing  
**Build:** Maven, Java 17+

## What works

- Shared JS `policy.yaml` + `switchboard.json`
- Switchboard hard DENY (unknown / not whitelisted / action denied)
- Credibility flags for policy
- TL-PX-shaped decision JSON
- JUnit tests

## Not yet

- Sealed audit JSONL
- Operator resolve / executeAuthorized
- HTTP service (use Go `tlpxd` for control plane)

## Commands

```bash
cd implementations/java

mvn test

mvn -q exec:java -Dexec.args="evaluate ../javascript/examples/intent-safe.json ../javascript/config/policy.yaml ../javascript/config/switchboard.json"

mvn -q exec:java -Dexec.args="evaluate ../javascript/examples/intent-unknown-agent.json ../javascript/config/policy.yaml ../javascript/config/switchboard.json"
```
