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

fail() {
  echo "FAIL  $*" >&2
  exit 1
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
  exit "$status"
}

write_report() {
  status=$1
  output=$2
  cleanup_status=$3
  run_time=$(/bin/date -u '+%Y-%m-%dT%H:%M:%SZ')
  stamp=$(/bin/date -u '+%Y-%m-%d-%H%M%S')
  report=$REPORT_DIR/slice-3.9-administrator-gate-$stamp.md
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

  {
    echo "# Slice 3.9 administrator gate — $run_time"
    echo
    echo "**Disposition:** $disposition"
    echo "**Base HEAD:** \`$head\`"
    echo "**Candidate state:** $candidate_state"
    echo "**Temporary-account cleanup status:** $cleanup_status"
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
  fi
  /bin/chmod 0644 "$report"
  echo "REPORT $report"
}

case ${1:-} in
  "") ;;
  --check)
    [ "$#" -eq 1 ] || fail "usage: $0 [--check]"
    check_prerequisites
    echo "READY $BINARY"
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
if /usr/bin/env \
  PATH=/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin \
  TLPX_PEP_USER=$PEP_ACCOUNT \
  TLPX_AGENT_USER=$AGENT_ACCOUNT \
  TLPX_OTHER_USER=$OTHER_ACCOUNT \
  "$HARNESS" "$BINARY" >"$RUN_OUTPUT" 2>&1; then
  test_status=0
else
  test_status=$?
fi

cleanup_status=0
cleanup_accounts || cleanup_status=$?

/bin/cat "$RUN_OUTPUT"
write_report "$test_status" "$RUN_OUTPUT" "$cleanup_status"

/bin/rm -f -- "$RUN_OUTPUT"
RUN_OUTPUT=
trap - EXIT HUP INT TERM

if [ "$test_status" -ne 0 ] || [ "$cleanup_status" -ne 0 ]; then
  exit 1
fi

echo "PASS  Northstar slice 3.9 administrator gate"
