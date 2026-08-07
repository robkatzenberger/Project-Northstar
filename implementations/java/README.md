# Java implementation (skeleton)

**Status:** Early skeleton — policy evaluate only  
**Build:** Maven, Java 17+

## What works

- Load shared JS reference `policy.yaml`
- Evaluate intents → `ALLOW` | `REQUIRE_APPROVAL` (TL-PX-shaped decision map)
- JUnit tests for safe allow + PII email rule

## Not yet

- Switchboard
- Sealed audit JSONL
- Operator / executeAuthorized
- Spring/HTTP service

## Commands

```bash
cd implementations/java

mvn test

mvn -q exec:java -Dexec.args="evaluate ../javascript/examples/intent-safe.json ../javascript/config/policy.yaml"

mvn -q exec:java -Dexec.args="evaluate ../javascript/examples/intent-pii-email.json ../javascript/config/policy.yaml"
```

## Layout

```text
java/
  pom.xml
  src/main/java/ai/trustlayer/tlpx/{policy,gate,cli}/
  src/test/java/...
  README.md
```
