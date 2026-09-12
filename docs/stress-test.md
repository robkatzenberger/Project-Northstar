# Ten-agent Switchboard stress test

This experiment runs ten concurrent Node.js agent processes against one gate
process using the real [Switchboard](../src/switchboard.js). Every agent has a
unique enrolled ID and its own temporary credential. The gate uses ten saved
registry snapshots as the experiment's whitelist database. These are test data;
the sample registry and previous live-test records are separate.

From `apex/`:

```sh
node test/stress/run.mjs
node test/stress/verify.mjs test/reports/<run-directory>
node test/stress/check-verifier.mjs test/reports/<run-directory>
```

The [completed first run](../test/reports/stress-20260910T042441767Z-25990a2a.md)
records all ten rounds, each agent's results, latency changes, and twelve checks
that deliberately corrupt copied evidence to challenge the verifier.

Each invocation creates a new evidence directory. Credentials are generated in
memory and delivered only to their corresponding worker through local IPC.
They are never written into the evidence. Workers receive neither the whitelist
membership nor an expected decision; their logs record actual observations.

## Experiment

- Ten agent processes remain alive concurrently for all ten rounds.
- Each round chooses a uniform random whitelist count from zero through ten,
  then randomly selects that many enrolled IDs without replacement. Draws are
  saved before the first ping, with no rerolls to select attractive results.
- All ten agents have the same exact test action/target grant. Each sends 100
  pings per round, so whitelist membership determines `PASS` or
  `DENY / NOT_WHITELISTED` for each of the 10,000 planned pings.
- Per-agent intervals decrease through 50, 35, 25, 18, 12, 8, 5, 3, 1, and 0 ms.
  Zero means no intentional delay. Sends follow an absolute schedule and do
  not wait for earlier responses, so slow responses cannot quietly reduce
  requested load.
- A prepare barrier starts each round only after every worker is ready and
  the new registry instance is installed. All responses and send callbacks
  drain before another registry is installed. A 30-second watchdog bounds each
  round; failures retain partial evidence.

Requested pacing and achieved throughput are separate measurements. Timer
precision, process scheduling, authentication, IPC, queuing, and synchronous
evidence writes all contribute to this local experiment. Agent round-trip
latency includes this overhead; gate evaluation duration measures only the
Switchboard call. Each duration uses a monotonic clock inside one process.
Faster requested pacing need not produce a faster completed round.

## Evidence

| File in each run directory | Contents |
| --- | --- |
| `manifest.json` | Exact plan, random draws, runtime/host context, base Git HEAD and working-tree status, source SHA-256 hashes |
| `registry-round-*.json` | Ten immutable registry snapshots, with every enrolled ID, whitelist flag, and action grant |
| `roster.json` | Ten actual worker identities and process IDs |
| `gate.jsonl` | Every received ping's decision, evaluation duration, and IPC response-send completion |
| `agents/*.jsonl` | Each worker's sends, received decisions, latency, and completion records |
| `agents/*.summary.json` | Each worker's own round counts, observed result codes, pacing lag, and maximum outstanding requests |
| `run-status.json` | Round completion times and worker exit statuses |
| `verification.json` | Independent offline reconciliation and per-round metrics |
| `credential-scan.json` | Counts from checking the in-memory credentials against completed evidence files |
| `evidence-sha256.json` | Hash inventory of completed run evidence; local integrity aid, not an external signature |

The verifier does not import the Switchboard implementation. It derives expected
results from the saved plan and requires one agent send, one gate decision, one
gate response-send completion, and one agent response for every planned ping.
It checks identity, registry version, exact action/target, result contents, and
duplicates or omissions. A successful gate send alone is not delivery evidence.

These records are independently written by separate test processes on the same
host. They are neither independent security acceptance nor sealed audit records.
The experiment exercises cooperative transport and whitelist screening. `PASS`
permits progression to constraint screening only; the test issues no execution
permission and makes no claim about human verification, hostile-process
isolation, network operation, or a production database. Random rounds do not
guarantee coverage of every whitelist size or combination.
