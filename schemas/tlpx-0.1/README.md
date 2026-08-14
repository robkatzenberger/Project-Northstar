# TL-PX 0.1 schemas

**Status:** Frozen historical evidence (2026-08-14)

These schemas define the TL-PX 0.1 Minimum Profile record shapes. They MUST NOT be edited to absorb 0.2 breaking changes.

Known frozen mismatch: `decision.schema.json` permits `ALLOW` and `REQUIRE_APPROVAL` only. The 0.1 JS runtime can emit Switchboard `DENY`. That defect is documented in `tests/reports/northstar-two-agent-test-proof.md` and is corrected on the 0.2 line, not here.

v0.2 schemas belong in `../tlpx-0.2/` (slice 2.3).
