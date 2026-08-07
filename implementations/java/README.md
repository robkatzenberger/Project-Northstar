# Java implementation

**Status:** Placeholder — not started  

Intended use: JVM shops (banks, large internal platforms) that standardize on Java/Kotlin services.

## Planned shape (draft)

```text
java/
  pom.xml or build.gradle.kts
  src/main/java/.../tlpx/
  src/test/java/...
```

## Requirements when implemented

- Conform to [TL-PX SPEC v0.1](../../docs/standard/SPEC-v0.1.md)
- Same decision / receipt / execution semantics as the JS reference
- Prefer Kotlin only if the team standard is Kotlin; either is fine if records match schemas

## Note

Java is **optional** for enterprise adoption of the *standard*. Many enterprises can consume a **Go or Node service** via HTTP. A Java port is for native JVM embedding or mandatory JVM stacks.
