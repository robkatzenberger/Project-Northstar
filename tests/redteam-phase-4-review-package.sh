#!/bin/sh
set -eu

PHASE4_REDTEAM_ROOT=$(git rev-parse --show-toplevel)
cd "$PHASE4_REDTEAM_ROOT"

PHASE4_REDTEAM_PACKAGE=docs/reviews/phase-4-external-security-review-readiness-2026-08-27.md
PHASE4_REDTEAM_SCOPE=docs/reviews/phase-4-review-scope.txt
PHASE4_REDTEAM_TMP=$(mktemp -d "${TMPDIR:-/tmp}/northstar-phase4-review.XXXXXX")
trap 'rm -r -- "$PHASE4_REDTEAM_TMP"' EXIT HUP INT TERM

expect_rejected() {
  if ./tests/check-phase-4-review-package.sh "$1" "$2" >/dev/null 2>&1; then
    echo "Phase 4 review package red team: FAIL — accepted $3" >&2
    exit 1
  fi
}

./tests/check-phase-4-review-package.sh >/dev/null

sed 's/64d08203c2f30964b176be8872b390ba3c28e5e6/0000000000000000000000000000000000000000/g' \
  "$PHASE4_REDTEAM_PACKAGE" >"$PHASE4_REDTEAM_TMP/wrong-target.md"
expect_rejected "$PHASE4_REDTEAM_TMP/wrong-target.md" "$PHASE4_REDTEAM_SCOPE" "wrong review target"

sed '/was not run/d' "$PHASE4_REDTEAM_PACKAGE" >"$PHASE4_REDTEAM_TMP/missing-gate.md"
expect_rejected "$PHASE4_REDTEAM_TMP/missing-gate.md" "$PHASE4_REDTEAM_SCOPE" "missing 3.9 non-claim"

sed '$d' "$PHASE4_REDTEAM_SCOPE" >"$PHASE4_REDTEAM_TMP/incomplete-scope.txt"
expect_rejected "$PHASE4_REDTEAM_PACKAGE" "$PHASE4_REDTEAM_TMP/incomplete-scope.txt" "incomplete scope manifest"

cp "$PHASE4_REDTEAM_PACKAGE" "$PHASE4_REDTEAM_TMP/false-acceptance.md"
printf '\nPhase 4 is accepted.\n' >>"$PHASE4_REDTEAM_TMP/false-acceptance.md"
expect_rejected "$PHASE4_REDTEAM_TMP/false-acceptance.md" "$PHASE4_REDTEAM_SCOPE" "false acceptance claim"

echo "Phase 4 review package red team: PASS (4/4 tampering cases rejected)"
