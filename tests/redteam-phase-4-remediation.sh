#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RUST_DIR="$ROOT/implementations/rust"
REPEATS=${TLPX_REMEDIATION_REPEATS:-5}
HANDOFF_REPEATS=${TLPX_HANDOFF_REPEATS:-20}

case $REPEATS in
  ''|*[!0-9]*) echo "TLPX_REMEDIATION_REPEATS must be an integer" >&2; exit 1 ;;
esac
case $HANDOFF_REPEATS in
  ''|*[!0-9]*) echo "TLPX_HANDOFF_REPEATS must be an integer" >&2; exit 1 ;;
esac
[ "$REPEATS" -ge 1 ] && [ "$REPEATS" -le 20 ] || {
  echo "TLPX_REMEDIATION_REPEATS must be within 1..20" >&2
  exit 1
}
[ "$HANDOFF_REPEATS" -ge 1 ] && [ "$HANDOFF_REPEATS" -le 100 ] || {
  echo "TLPX_HANDOFF_REPEATS must be within 1..100" >&2
  exit 1
}

iteration=1
while [ "$iteration" -le "$REPEATS" ]; do
  (
    cd "$RUST_DIR"
    cargo test --offline --test authority_mvp switchboard_refusal_precedes_tenant_policy_activation
    cargo test --offline --test authority_mvp authorization_mac_key_revocation_after_claim_blocks_execution_start
    cargo test --offline --test authority_mvp authorization_mac_key_revocation_and_execution_start_have_a_serial_order
    cargo test --offline --test evidence_outbox authority_denies_transition_before_outbox_capacity_can_be_exceeded
    cargo test --offline --test evidence_outbox combined_readiness_rejects_database_acknowledgement_without_sink_bytes
    cargo test --offline --test evidence_outbox forged_database_export_acknowledgement_fails_hmac_reconciliation
    cargo test --offline --test evidence_outbox reconciliation_detects_operational_source_row_tampering
    cargo test --offline --test evidence_outbox reopening_rejects_a_dropped_authority_unique_index
    cargo test --offline --test evidence_outbox reopening_rejects_a_weakened_idempotency_index_predicate
    cargo test --offline --lib shell_runner::executable_provenance_tests
  )
  iteration=$((iteration + 1))
done

iteration=1
while [ "$iteration" -le "$HANDOFF_REPEATS" ]; do
  (
    cd "$RUST_DIR"
    cargo test --offline --test multi_agent_handoff
  )
  iteration=$((iteration + 1))
done

sh -n "$RUST_DIR/scripts/restricted-agent-acceptance.sh"
"$ROOT/tests/check-operational-profile.sh"

echo "Phase 4 remediation red team: PASS ($REPEATS focused repetitions; $HANDOFF_REPEATS handoff repetitions)"
