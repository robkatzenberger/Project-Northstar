#!/bin/sh
set -eu

umask 077

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
RUST_DIR=$ROOT/implementations/rust
HARNESS=$RUST_DIR/scripts/restricted-agent-acceptance.sh
BINARY=$RUST_DIR/target/debug/tlpx-run
REPORT_DIR=$ROOT/tests/reports

PEP_ACCOUNT=_tlpx39_pep
AGENT_ACCOUNT=_tlpx39_agent
OTHER_ACCOUNT=_tlpx39_other
PEP_UID=61001
AGENT_UID=61002
OTHER_UID=61003

CREATED_ACCOUNTS=
RUN_OUTPUT=
RUN_EVIDENCE=
BINARY_SHA256=

fail() {
  echo "FAIL  $*" >&2
  exit 1
}

file_sha256() {
  digest=$(/usr/bin/shasum -a 256 "$1" | /usr/bin/awk '{print $1}')
  case $digest in
    *[!0-9a-f]*|'') fail "SHA-256 is malformed for $1" ;;
  esac
  [ "${#digest}" -eq 64 ] || fail "SHA-256 has the wrong length for $1"
  printf 'sha256:%s\n' "$digest"
}

account_available() {
  account=$1
  uid=$2

  if /usr/bin/dscl . -read "/Users/$account" >/dev/null 2>&1; then
    fail "refusing to reuse existing directory-service account $account"
  fi
  if /usr/bin/id "$account" >/dev/null 2>&1; then
    fail "refusing to reuse existing account $account"
  fi
  if /usr/bin/dscacheutil -q user -a uid "$uid" | /usr/bin/grep -q '^name:'; then
    fail "refusing to reuse existing UID $uid"
  fi
}

check_prerequisites() {
  [ "$(/usr/bin/uname -s)" = Darwin ] || fail "this one-command wrapper supports macOS only"
  [ -x /usr/bin/dscl ] || fail "macOS directory service tool is unavailable"
  [ -x /usr/bin/dscacheutil ] || fail "macOS directory cache tool is unavailable"
  [ -x /usr/bin/shasum ] || fail "macOS SHA-256 tool is unavailable"
  [ -x /usr/sbin/chown ] || fail "macOS ownership tool is unavailable"
  [ -x /usr/local/bin/node ] || fail "Node.js is required at /usr/local/bin/node"
  [ -x "$HARNESS" ] || fail "acceptance harness is missing or not executable: $HARNESS"
  [ -x "$BINARY" ] || fail "tlpx-run is not built: $BINARY"
  /bin/sh -n "$HARNESS" || fail "acceptance harness has invalid shell syntax"

  stale=$(
    /usr/bin/find "$RUST_DIR/src" "$RUST_DIR/Cargo.toml" "$RUST_DIR/Cargo.lock" \
      -newer "$BINARY" -print -quit
  )
  [ -z "$stale" ] || fail "tlpx-run is older than $stale; rebuild it before the administrator gate"
  BINARY_SHA256=$(file_sha256 "$BINARY")

  account_available "$PEP_ACCOUNT" "$PEP_UID"
  account_available "$AGENT_ACCOUNT" "$AGENT_UID"
  account_available "$OTHER_ACCOUNT" "$OTHER_UID"
}

create_inactive_account() {
  account=$1
  uid=$2

  account_available "$account" "$uid"
  CREATED_ACCOUNTS="$account $CREATED_ACCOUNTS"
  /usr/bin/dscl . -create "/Users/$account"
  /usr/bin/dscl . -create "/Users/$account" UniqueID "$uid"
  /usr/bin/dscl . -create "/Users/$account" PrimaryGroupID 20
  /usr/bin/dscl . -create "/Users/$account" RealName "Northstar 3.9 temporary test identity"
  /usr/bin/dscl . -create "/Users/$account" NFSHomeDirectory /var/empty
  /usr/bin/dscl . -create "/Users/$account" UserShell /usr/bin/false
  /usr/bin/dscl . -create "/Users/$account" IsHidden 1
  /usr/bin/dscl . -create "/Users/$account" Password '*'

  attempt=0
  while ! /usr/bin/id "$account" >/dev/null 2>&1; do
    attempt=$((attempt + 1))
    [ "$attempt" -lt 50 ] || fail "timed out waiting for $account to become visible"
    /bin/sleep 0.1
  done
}

cleanup_accounts() {
  failed=0

  for account in $CREATED_ACCOUNTS; do
    if /usr/bin/dscl . -read "/Users/$account" >/dev/null 2>&1; then
      if ! /usr/bin/dscl . -delete "/Users/$account" >/dev/null 2>&1; then
        echo "FAIL  could not remove temporary account $account" >&2
        failed=1
      fi
    fi
  done
  /usr/bin/dscacheutil -flushcache >/dev/null 2>&1 || failed=1

  for account in $CREATED_ACCOUNTS; do
    if /usr/bin/dscl . -read "/Users/$account" >/dev/null 2>&1 || \
      /usr/bin/id "$account" >/dev/null 2>&1; then
      echo "FAIL  temporary account remains after cleanup: $account" >&2
      failed=1
    fi
  done

  return "$failed"
}

emergency_cleanup() {
  status=$?
  trap - EXIT HUP INT TERM
  cleanup_accounts || status=1
  if [ -n "$RUN_OUTPUT" ] && [ -f "$RUN_OUTPUT" ]; then
    /bin/rm -f -- "$RUN_OUTPUT"
  fi
  if [ -n "$RUN_EVIDENCE" ] && [ -f "$RUN_EVIDENCE" ]; then
    /bin/rm -f -- "$RUN_EVIDENCE"
  fi
  exit "$status"
}

write_report() {
  status=$1
  output=$2
  cleanup_status=$3
  run_time=$(/bin/date -u '+%Y-%m-%dT%H:%M:%SZ')
  stamp=$(/bin/date -u '+%Y-%m-%d-%H%M%S')
  report=$REPORT_DIR/slice-3.9-administrator-gate-$stamp.md
  evidence_name=slice-3.9-administrator-gate-$stamp.evidence.jsonl
  evidence_report=$REPORT_DIR/$evidence_name
  head=$(/usr/bin/git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo unknown)
  if [ -n "$(/usr/bin/git -C "$ROOT" status --porcelain 2>/dev/null || true)" ]; then
    candidate_state="dirty working tree; no exact candidate commit"
  else
    candidate_state="clean exact commit $head"
  fi
  if [ "$status" -eq 0 ] && [ "$cleanup_status" -eq 0 ]; then
    disposition=PASS
  else
    disposition=FAIL
  fi
  evidence_digest=not-produced
  evidence_reference=not-produced
  if [ -n "$RUN_EVIDENCE" ] && [ -s "$RUN_EVIDENCE" ]; then
    /bin/cp "$RUN_EVIDENCE" "$evidence_report"
    evidence_digest=$(file_sha256 "$evidence_report")
    evidence_reference="[$evidence_name](./$evidence_name)"
  fi

  {
    echo "# Slice 3.9 administrator gate — $run_time"
    echo
    echo "**Disposition:** $disposition"
    echo "**Base HEAD:** \`$head\`"
    echo "**Candidate state:** $candidate_state"
    echo "**Tested binary SHA-256:** \`$BINARY_SHA256\`"
    echo "**Temporary-account cleanup status:** $cleanup_status"
    echo "**Retained canonical evidence:** $evidence_reference"
    echo "**Retained evidence SHA-256:** \`$evidence_digest\`"
    echo
    echo "This is a local administrator-backed acceptance run for the bounded macOS separate-identity profile. It is not independent review, Section 3 acceptance, production-readiness evidence, or a claim of universal forced mediation."
    echo
    echo "## Exact output"
    echo
    echo '```text'
    /bin/cat "$output"
    echo '```'
  } >"$report"

  if [ -n "${SUDO_UID:-}" ] && [ -n "${SUDO_GID:-}" ]; then
    /usr/sbin/chown "$SUDO_UID:$SUDO_GID" "$report"
    if [ -f "$evidence_report" ]; then
      /usr/sbin/chown "$SUDO_UID:$SUDO_GID" "$evidence_report"
    fi
  fi
  /bin/chmod 0644 "$report"
  if [ -f "$evidence_report" ]; then
    /bin/chmod 0644 "$evidence_report"
  fi
  echo "REPORT $report"
}

case ${1:-} in
  "") ;;
  --check)
    [ "$#" -eq 1 ] || fail "usage: $0 [--check]"
    check_prerequisites
    echo "READY $BINARY"
    echo "SHA256 $BINARY_SHA256"
    echo "RUN   sudo ./run-northstar-3.9.sh"
    exit 0
    ;;
  *) fail "usage: $0 [--check]" ;;
esac

[ "$(/usr/bin/id -u)" -eq 0 ] || fail "run this gate with sudo: sudo ./run-northstar-3.9.sh"
check_prerequisites

trap emergency_cleanup EXIT HUP INT TERM

create_inactive_account "$PEP_ACCOUNT" "$PEP_UID"
create_inactive_account "$AGENT_ACCOUNT" "$AGENT_UID"
create_inactive_account "$OTHER_ACCOUNT" "$OTHER_UID"

RUN_OUTPUT=$(/usr/bin/mktemp -t northstar-3.9-output)
RUN_EVIDENCE=$(/usr/bin/mktemp /tmp/northstar-3.9-evidence.XXXXXX)
if /usr/bin/env \
  PATH=/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin \
  TLPX_PEP_USER=$PEP_ACCOUNT \
  TLPX_AGENT_USER=$AGENT_ACCOUNT \
  TLPX_OTHER_USER=$OTHER_ACCOUNT \
  TLPX_EVIDENCE_OUT=$RUN_EVIDENCE \
  TLPX_EXPECTED_BINARY_SHA256=$BINARY_SHA256 \
  "$HARNESS" "$BINARY" >"$RUN_OUTPUT" 2>&1; then
  test_status=0
else
  test_status=$?
fi

if [ "$test_status" -eq 0 ] && [ ! -s "$RUN_EVIDENCE" ]; then
  echo "FAIL  slice 3.9 harness did not retain canonical evidence" >>"$RUN_OUTPUT"
  test_status=1
fi

cleanup_status=0
cleanup_accounts || cleanup_status=$?

/bin/cat "$RUN_OUTPUT"
write_report "$test_status" "$RUN_OUTPUT" "$cleanup_status"

/bin/rm -f -- "$RUN_OUTPUT"
RUN_OUTPUT=
/bin/rm -f -- "$RUN_EVIDENCE"
RUN_EVIDENCE=
trap - EXIT HUP INT TERM

if [ "$test_status" -ne 0 ] || [ "$cleanup_status" -ne 0 ]; then
  exit 1
fi

echo "PASS  Northstar slice 3.9 administrator gate"
