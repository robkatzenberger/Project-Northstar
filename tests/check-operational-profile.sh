#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PROFILE="$ROOT/docs/operations/README.md"
INCIDENT="$ROOT/docs/operations/INCIDENT-RESPONSE.md"

require_text() {
  file=$1
  text=$2
  if ! grep -Fq -- "$text" "$file"; then
    echo "operational profile missing required text: $text" >&2
    exit 1
  fi
}

test -f "$PROFILE"
test -f "$INCIDENT"

for heading in \
  "## Readiness decision" \
  "## Aggregate health surface" \
  "## Audit capacity and backpressure" \
  "## Backup and restore" \
  "## Change and rollback" \
  "## Incident response"
do
  require_text "$PROFILE" "$heading"
done

for topic in \
  "### Evidence, chain, or database integrity failure" \
  "### Stale exporter lock" \
  "### Torn or incomplete sink row" \
  "### Audit backlog or 64 MiB hard stop" \
  "### Authorization-signing or audit-sealing key compromise" \
  "### Trusted-time rollback or uncertainty" \
  "### Outcome unknown or reconciliation required" \
  "### Identity, configuration, adapter, or alternate-path bypass" \
  "## Recovery gate"
do
  require_text "$INCIDENT" "$topic"
done

require_text "$PROFILE" "production HA"
require_text "$PROFILE" "no safe rotation implementation"
require_text "$INCIDENT" "Never automate age-based lock deletion."
require_text "$INCIDENT" "disable automatic retry"
require_text "$INCIDENT" "does not convert builder verification into independent acceptance"

if grep -Eqi -- 'rm[[:space:]]+-rf|DELETE[[:space:]]+FROM[[:space:]]+tlpx_|UPDATE[[:space:]]+tlpx_' "$PROFILE" "$INCIDENT"; then
  echo "operational profile contains a destructive database/filesystem instruction" >&2
  exit 1
fi

echo "Operational profile check: PASS"
