#!/bin/sh
set -eu

umask 077

RUST_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
REPO_DIR=$(CDPATH= cd -- "$RUST_DIR/../.." && pwd)
SOURCE_BIN=${1:-"$RUST_DIR/target/debug/tlpx-run"}

PEP_PID=
EXPIRY_PID=
RESTART_PID=

fail() {
  echo "FAIL  $*" >&2
  exit 1
}

CHOWN=$(command -v chown 2>/dev/null || true)
if [ -z "$CHOWN" ] && [ -x /usr/sbin/chown ]; then
  CHOWN=/usr/sbin/chown
fi
[ -x "$CHOWN" ] || fail "slice 3.9 acceptance requires chown"

pass() {
  echo "PASS  $*"
}

file_sha256() {
  file=$1
  if command -v shasum >/dev/null 2>&1; then
    digest=$(shasum -a 256 "$file" | awk '{print $1}')
  elif command -v sha256sum >/dev/null 2>&1; then
    digest=$(sha256sum "$file" | awk '{print $1}')
  else
    fail "slice 3.9 acceptance requires shasum or sha256sum"
  fi
  case $digest in
    *[!0-9a-f]*|'') fail "tested binary SHA-256 is malformed" ;;
  esac
  [ "${#digest}" -eq 64 ] || fail "tested binary SHA-256 has the wrong length"
  printf 'sha256:%s\n' "$digest"
}

cleanup() {
  for pid in "$PEP_PID" "$EXPIRY_PID" "$RESTART_PID"; do
    if [ -n "$pid" ]; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  case ${ACCEPTANCE_ROOT:-} in
    /tmp/northstar-3.9.*) rm -rf -- "$ACCEPTANCE_ROOT" ;;
  esac
}

trap cleanup EXIT HUP INT TERM

[ "$(id -u)" -eq 0 ] || fail "slice 3.9 acceptance requires root to enter separate numeric UIDs"
case $(uname -s) in
  Linux)
    command -v setpriv >/dev/null 2>&1 || fail "Linux acceptance requires setpriv"
    PEP_UID=61001
    PEP_GID=61001
    PEP_OWNER=$PEP_UID
    PEP_GROUP=$PEP_GID
    AGENT_UID=61002
    AGENT_GID=61002
    AGENT_OWNER=$AGENT_UID
    AGENT_GROUP=$AGENT_GID
    OTHER_UID=61003
    OTHER_GID=61003
    for uid in "$PEP_UID" "$AGENT_UID" "$OTHER_UID"; do
      if ps -eo uid= | awk -v wanted="$uid" '$1 == wanted { found=1 } END { exit !found }'; then
        fail "numeric UID $uid is already active"
      fi
    done
    ;;
  Darwin)
    command -v sudo >/dev/null 2>&1 || fail "macOS acceptance requires sudo"
    [ -n "${TLPX_PEP_USER:-}" ] || fail "set TLPX_PEP_USER to a dedicated inactive macOS test account"
    [ -n "${TLPX_AGENT_USER:-}" ] || fail "set TLPX_AGENT_USER to a distinct dedicated inactive macOS test account"
    [ -n "${TLPX_OTHER_USER:-}" ] || fail "set TLPX_OTHER_USER to a third dedicated inactive macOS test account"
    [ "$TLPX_PEP_USER" != "$TLPX_AGENT_USER" ] || fail "macOS test accounts must be distinct"
    [ "$TLPX_PEP_USER" != "$TLPX_OTHER_USER" ] || fail "macOS test accounts must be distinct"
    [ "$TLPX_AGENT_USER" != "$TLPX_OTHER_USER" ] || fail "macOS test accounts must be distinct"
    for account in "$TLPX_PEP_USER" "$TLPX_AGENT_USER" "$TLPX_OTHER_USER"; do
      case $account in
        root|_www|nobody|daemon) fail "refusing shared/system account $account; create dedicated test accounts" ;;
      esac
      id "$account" >/dev/null 2>&1 || fail "macOS test account $account does not exist"
      if [ -n "${SUDO_USER:-}" ] && [ "$account" = "$SUDO_USER" ]; then
        fail "macOS test accounts must not be the invoking user"
      fi
    done
    PEP_OWNER=$TLPX_PEP_USER
    PEP_GROUP=$(id -gn "$PEP_OWNER")
    AGENT_OWNER=$TLPX_AGENT_USER
    AGENT_GROUP=$(id -gn "$AGENT_OWNER")
    OTHER_OWNER=$TLPX_OTHER_USER
    OTHER_GROUP=$(id -gn "$OTHER_OWNER")
    PEP_UID=$(id -u "$PEP_OWNER")
    PEP_GID=$(id -g "$PEP_OWNER")
    AGENT_UID=$(id -u "$AGENT_OWNER")
    AGENT_GID=$(id -g "$AGENT_OWNER")
    OTHER_UID=$(id -u "$OTHER_OWNER")
    OTHER_GID=$(id -g "$OTHER_OWNER")
    for uid in "$PEP_UID" "$AGENT_UID" "$OTHER_UID"; do
      [ "$uid" -ne 0 ] || fail "macOS test accounts must not be root"
      if ps -eo uid= | awk -v wanted="$uid" '$1 == wanted { found=1 } END { exit !found }'; then
        fail "dedicated macOS test UID $uid is already active"
      fi
    done
    ;;
  *) fail "slice 3.9 acceptance supports Linux and macOS" ;;
esac
NODE=$(command -v node 2>/dev/null || true)
if [ -z "$NODE" ] && [ -x /usr/local/bin/node ]; then
  NODE=/usr/local/bin/node
fi
[ -x "$NODE" ] || fail "Node.js is required for independent schema validation"
[ -x "$SOURCE_BIN" ] || fail "build tlpx-run first: cargo build --offline --bin tlpx-run"

ACCEPTANCE_ROOT=$(mktemp -d /tmp/northstar-3.9.XXXXXX)
SERVICE_DIR=$ACCEPTANCE_ROOT/service
STATE_DIR=$ACCEPTANCE_ROOT/state
PROTECTED_DIR=$ACCEPTANCE_ROOT/protected
AGENT_DIR=$ACCEPTANCE_ROOT/agent
STAGED_BIN=$SERVICE_DIR/tlpx-run
SOCKET=$SERVICE_DIR/pep.sock
DATABASE=$STATE_DIR/authority.sqlite
KEY=$STATE_DIR/seal.key
CONFIG=$STATE_DIR/pep.conf
MARKER=$PROTECTED_DIR/only-marker

mkdir -p "$SERVICE_DIR" "$STATE_DIR" "$PROTECTED_DIR" "$AGENT_DIR"
chmod 0711 "$ACCEPTANCE_ROOT"
"$CHOWN" "$PEP_OWNER:$PEP_GROUP" "$SERVICE_DIR"
chmod 0755 "$SERVICE_DIR"
"$CHOWN" "$PEP_OWNER:$PEP_GROUP" "$STATE_DIR" "$PROTECTED_DIR"
chmod 0700 "$STATE_DIR" "$PROTECTED_DIR"
"$CHOWN" "$AGENT_OWNER:$AGENT_GROUP" "$AGENT_DIR"
chmod 0700 "$AGENT_DIR"
cp "$SOURCE_BIN" "$STAGED_BIN"
"$CHOWN" "$PEP_OWNER:$PEP_GROUP" "$STAGED_BIN"
chmod 0555 "$STAGED_BIN"
TESTED_BINARY_SHA256=$(file_sha256 "$STAGED_BIN")
if [ -n "${TLPX_EXPECTED_BINARY_SHA256:-}" ] && \
  [ "$TESTED_BINARY_SHA256" != "$TLPX_EXPECTED_BINARY_SHA256" ]; then
  fail "staged binary SHA-256 does not match the runner's clean-tree digest"
fi
echo "TESTED_BINARY_SHA256 $TESTED_BINARY_SHA256"
pass "staged tested binary matches reported SHA-256"
dd if=/dev/urandom of="$KEY" bs=64 count=1 2>/dev/null
"$CHOWN" "$PEP_OWNER:$PEP_GROUP" "$KEY"
chmod 0400 "$KEY"

write_config() {
  destination=$1
  database=$2
  socket=$3
  marker=$4
  maximum=$5
  claim_window=$6
  claim_delay=$7
  printf '%s\n' \
    "database=$database" \
    "socket=$socket" \
    "role_keys=$KEY" \
    "protected_marker=$marker" \
    "agent_uid=$AGENT_UID" \
    "agent_gid=$AGENT_GID" \
    "max_connections=$maximum" \
    "claim_window_ms=$claim_window" \
    "claim_delay_ms=$claim_delay" > "$destination"
  "$CHOWN" "$PEP_OWNER:$PEP_GROUP" "$destination"
  chmod 0400 "$destination"
}

as_pep() {
  case $(uname -s) in
    Darwin) sudo -n -u "$PEP_OWNER" -g "$PEP_GROUP" env -i PATH=/usr/bin:/bin "$@" ;;
    *) env -i PATH=/usr/bin:/bin setpriv --reuid="$PEP_UID" --regid="$PEP_GID" --clear-groups "$@" ;;
  esac
}

as_agent() {
  case $(uname -s) in
    Darwin) sudo -n -u "$AGENT_OWNER" -g "$AGENT_GROUP" env -i PATH=/usr/bin:/bin "$@" ;;
    *) env -i PATH=/usr/bin:/bin setpriv --reuid="$AGENT_UID" --regid="$AGENT_GID" --clear-groups "$@" ;;
  esac
}

as_other() {
  case $(uname -s) in
    Darwin) sudo -n -u "$OTHER_OWNER" -g "$OTHER_GROUP" env -i PATH=/usr/bin:/bin "$@" ;;
    *) env -i PATH=/usr/bin:/bin setpriv --reuid="$OTHER_UID" --regid="$OTHER_GID" --clear-groups "$@" ;;
  esac
}

file_uid() {
  case $(uname -s) in
    Darwin) stat -f %u "$1" ;;
    *) stat -c %u "$1" ;;
  esac
}

file_mtime() {
  case $(uname -s) in
    Darwin) stat -f %m "$1" ;;
    *) stat -c %Y "$1" ;;
  esac
}

wait_for_socket() {
  socket=$1
  pid=$2
  count=0
  while [ ! -S "$socket" ]; do
    if ! kill -0 "$pid" 2>/dev/null; then
      fail "PEP exited before binding $socket"
    fi
    count=$((count + 1))
    [ "$count" -lt 500 ] || fail "timed out waiting for $socket"
    sleep 0.01
  done
}

expect_contains() {
  actual=$1
  expected=$2
  label=$3
  case $actual in
    *"$expected"*) pass "$label" ;;
    *) fail "$label: expected $expected, got $actual" ;;
  esac
}

if as_agent /usr/bin/touch "$MARKER" 2>/dev/null; then
  fail "restricted agent wrote protected target directly"
fi
pass "restricted agent cannot write protected target directly"

if as_agent /bin/sh -c "printf x > '$MARKER'" 2>/dev/null; then
  fail "alternate shell route wrote protected target"
fi
pass "alternate shell route cannot write protected target"

as_agent /bin/ln -s "$MARKER" "$AGENT_DIR/marker-alias"
if as_agent /usr/bin/touch "$AGENT_DIR/marker-alias" 2>/dev/null; then
  fail "alternate symlink path wrote protected target"
fi
pass "alternate symlink path cannot write protected target"

if as_agent /bin/sh -c "printf x >> '$STAGED_BIN'" 2>/dev/null; then
  fail "restricted agent modified PEP code"
fi
pass "restricted agent cannot modify PEP code"

write_config "$CONFIG" "$DATABASE" "$SOCKET" "$MARKER" 7 5000 0
[ "$(file_uid "$KEY")" -eq "$PEP_UID" ] || fail "PEP role-key bundle is not service-owned"
[ "$(file_uid "$PROTECTED_DIR")" -eq "$PEP_UID" ] || fail "protected capability is not service-owned"
as_pep "$STAGED_BIN" serve "$CONFIG" >"$STATE_DIR/pep.stdout" 2>"$STATE_DIR/pep.stderr" &
PEP_PID=$!
wait_for_socket "$SOCKET" "$PEP_PID"

if as_agent /bin/cat "$CONFIG" >/dev/null 2>&1; then
  fail "restricted agent read PEP configuration"
fi
if as_agent /bin/cat "$KEY" >/dev/null 2>&1; then
  fail "restricted agent read PEP role-key bundle"
fi
if as_agent /bin/sh -c "printf x > '$SERVICE_DIR/replacement'" 2>/dev/null; then
  fail "restricted agent modified PEP service directory"
fi
pass "PEP configuration, key, and endpoint directory resist agent access"

response=$(as_other "$STAGED_BIN" request "$SOCKET" other-1 CREATE_MARKER)
expect_contains "$response" "DENY AUTHENTICATION_FAILED" "second UID cannot use the mapped agent identity"

response=$(as_agent "$STAGED_BIN" raw "$SOCKET" "EXECUTE	forged-1	CREATE_MARKER	principal=restricted.pep")
expect_contains "$response" "DENY PEP_REQUEST_INVALID" "caller-supplied principal field is rejected"

response=$(as_agent "$STAGED_BIN" request "$SOCKET" denied-1 DELETE_MARKER)
expect_contains "$response" "DENY SWITCHBOARD_ACTION_DENIED" "unsupported action is denied before authorization"
[ ! -e "$MARKER" ] || fail "denied request created the protected marker"

response=$(as_agent "$STAGED_BIN" request "$SOCKET" allowed-1 CREATE_MARKER)
expect_contains "$response" "OK COMPLETED" "authenticated exact request completes"
[ -f "$MARKER" ] || fail "valid request did not create the protected marker"
[ "$(file_uid "$MARKER")" -eq "$PEP_UID" ] || fail "protected marker is not PEP-owned"
pass "PEP identity owns the protected side effect"

response=$(as_agent "$STAGED_BIN" request "$SOCKET" allowed-1 CREATE_MARKER)
expect_contains "$response" "DENY PEP_PROTECTED_TARGET_EXISTS" "existing protected target blocks a repeat request before evaluation"

response=$(as_agent "$STAGED_BIN" request "$SOCKET" mutated-2 CREATE_MARKER)
expect_contains "$response" "DENY PEP_PROTECTED_TARGET_EXISTS" "existing protected target blocks a new request"

as_pep /bin/rm "$MARKER"
[ ! -e "$MARKER" ] || fail "PEP-owned test cleanup did not remove the protected marker"
response=$(as_agent "$STAGED_BIN" request "$SOCKET" allowed-1 CREATE_MARKER)
expect_contains "$response" "DENY ALREADY_CLAIMED" "consumed authorization cannot be replayed after PEP-owned target deletion"
[ ! -e "$MARKER" ] || fail "consumed authorization replay recreated the protected marker"
as_pep /usr/bin/touch "$MARKER"
[ "$(file_uid "$MARKER")" -eq "$PEP_UID" ] || fail "PEP-owned restart fixture has the wrong owner"

response=$(as_agent "$STAGED_BIN" raw "$SOCKET" "EXECUTE	path-1	CREATE_MARKER	/tmp/alternate")
expect_contains "$response" "DENY PEP_REQUEST_INVALID" "caller cannot inject an alternate path or argv"

wait "$PEP_PID"
PEP_PID=
[ ! -e "$SOCKET" ] || fail "PEP did not remove its endpoint after bounded shutdown"
pass "unauthenticated peer does not consume the authenticated-work budget"

response=$(as_agent "$STAGED_BIN" probe-bind "$SOCKET")
expect_contains "$response" "DENY PEP_REPLACEMENT_BIND" "restricted agent cannot bind a replacement endpoint"

EVIDENCE=$STATE_DIR/evidence.jsonl
as_pep "$STAGED_BIN" verify "$CONFIG" >"$EVIDENCE" 2>"$STATE_DIR/verify.log"
"$NODE" "$REPO_DIR/implementations/javascript/scripts/validate-pep-evidence.mjs" "$EVIDENCE"
if [ -n "${TLPX_EVIDENCE_OUT:-}" ]; then
  case $TLPX_EVIDENCE_OUT in
    /tmp/northstar-3.9-evidence.*) ;;
    *) fail "retained evidence output must be a scoped /tmp path" ;;
  esac
  [ -f "$TLPX_EVIDENCE_OUT" ] || fail "retained evidence output was not pre-created"
  [ ! -L "$TLPX_EVIDENCE_OUT" ] || fail "retained evidence output must not be a symlink"
  cp "$EVIDENCE" "$TLPX_EVIDENCE_OUT"
  chmod 0600 "$TLPX_EVIDENCE_OUT"
fi
pass "sealed execution and denial evidence independently reconciles and validates"

before_restart=$(file_mtime "$MARKER")
sleep 1
RESTART_CONFIG=$STATE_DIR/restart.conf
write_config "$RESTART_CONFIG" "$DATABASE" "$SOCKET" "$MARKER" 1 5000 0
as_pep "$STAGED_BIN" serve "$RESTART_CONFIG" >"$STATE_DIR/restart.stdout" 2>"$STATE_DIR/restart.stderr" &
RESTART_PID=$!
wait_for_socket "$SOCKET" "$RESTART_PID"
response=$(as_agent "$STAGED_BIN" request "$SOCKET" restart-1 CREATE_MARKER)
expect_contains "$response" "DENY PEP_PROTECTED_TARGET_EXISTS" "restart does not reopen completed target"
wait "$RESTART_PID"
RESTART_PID=
[ "$(file_mtime "$MARKER")" = "$before_restart" ] || fail "restart changed the protected marker"

EXPIRY_SOCKET=$SERVICE_DIR/expiry.sock
EXPIRY_DATABASE=$STATE_DIR/expiry.sqlite
EXPIRY_MARKER=$PROTECTED_DIR/expiry-marker
EXPIRY_CONFIG=$STATE_DIR/expiry.conf
write_config "$EXPIRY_CONFIG" "$EXPIRY_DATABASE" "$EXPIRY_SOCKET" "$EXPIRY_MARKER" 1 10 50
as_pep "$STAGED_BIN" serve "$EXPIRY_CONFIG" >"$STATE_DIR/expiry.stdout" 2>"$STATE_DIR/expiry.stderr" &
EXPIRY_PID=$!
wait_for_socket "$EXPIRY_SOCKET" "$EXPIRY_PID"
response=$(as_agent "$STAGED_BIN" request "$EXPIRY_SOCKET" expired-1 CREATE_MARKER)
expect_contains "$response" "DENY AUTHORIZATION_EXPIRED" "expired authorization cannot start the side effect"
wait "$EXPIRY_PID"
EXPIRY_PID=
[ ! -e "$EXPIRY_MARKER" ] || fail "expired authorization created a marker"

pass "slice 3.9 separate-identity restricted-agent acceptance"
