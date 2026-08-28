#!/bin/sh
set -eu

PHASE4_ROOT=$(git rev-parse --show-toplevel)
cd "$PHASE4_ROOT"

PHASE4_PACKAGE=${1:-docs/reviews/phase-4-external-security-review-readiness-2026-08-27.md}
PHASE4_SCOPE=${2:-docs/reviews/phase-4-review-scope.txt}
PHASE4_BASE=97e24eefe7786a2c5f88ea4c2d295e6b35e08bd9
PHASE4_TARGET=64d08203c2f30964b176be8872b390ba3c28e5e6

fail() {
  echo "Phase 4 review package: FAIL — $1" >&2
  exit 1
}

require_text() {
  grep -Fq "$2" "$1" || fail "missing required text: $2"
}

test -f "$PHASE4_PACKAGE" || fail "review package is missing"
test -f "$PHASE4_SCOPE" || fail "review scope manifest is missing"
git cat-file -e "$PHASE4_BASE^{commit}" || fail "base commit is unavailable"
git cat-file -e "$PHASE4_TARGET^{commit}" || fail "target commit is unavailable"
git merge-base --is-ancestor "$PHASE4_BASE" "$PHASE4_TARGET" || fail "target does not descend from base"

for PHASE4_COMMIT in \
  980327d76a2b9e157bb52daeef9bf3a05e3e0d34 \
  efa7f0f71357e922c7dc65b2a379cbb11d04ff3a \
  fb776786e211523ae0d1697f28475fa4780f12cb \
  833d8d4a01f6a63b8f9ed06c20a531f26bdcc371 \
  40fb8d8e85aee1e6526b9365c9f3a214fdf7a2ec \
  eb0e624dd9f94672a9dab825f1e01df7c6a0dab8 \
  74ff502d5e73d20cfb2e857d68b73f4964cf1dc0 \
  bdd8a00b7cc6e97b80f36c34ea8dad7e8cfb879b \
  8ff813b064bd52d3ac7b91c3241fb56da70859d2 \
  536111a483b3bece113c7b4e73148a54f74fe9c5 \
  64d08203c2f30964b176be8872b390ba3c28e5e6
do
  git merge-base --is-ancestor "$PHASE4_COMMIT" "$PHASE4_TARGET" ||
    fail "listed commit is not in the target history: $PHASE4_COMMIT"
  require_text "$PHASE4_PACKAGE" "$PHASE4_COMMIT"
done

git diff --name-only "$PHASE4_BASE..$PHASE4_TARGET" |
  diff -u "$PHASE4_SCOPE" - >/dev/null || fail "changed-file manifest does not match target range"

for PHASE4_REPORT in \
  tests/reports/slice-4.1-separated-key-roles-builder-verification-2026-08-27.md \
  tests/reports/slice-4.2-durable-audit-export-builder-verification-2026-08-27.md \
  tests/reports/slice-4.3-race-time-crash-assurance-builder-verification-2026-08-27.md \
  tests/reports/slice-4.4-operational-readiness-builder-verification-2026-08-27.md \
  tests/reports/slice-4.5-multi-agent-handoff-builder-verification-2026-08-27.md
do
  test -f "$PHASE4_REPORT" || fail "builder report is missing: $PHASE4_REPORT"
  require_text "$PHASE4_PACKAGE" "$PHASE4_REPORT"
done

for PHASE4_REQUIRED in \
  "not independently accepted" \
  "sudo ./scripts/restricted-agent-acceptance.sh" \
  "was not run" \
  "forced mediation" \
  "Portable revocation" \
  "formal model checking" \
  "no bearer" \
  "cargo test --all-targets --offline" \
  "npm run redteam" \
  "external security review readiness package" \
  "Required reviewer challenges" \
  "Expected independent deliverable"
do
  require_text "$PHASE4_PACKAGE" "$PHASE4_REQUIRED"
done

if grep -Fq "Phase 4 is accepted" "$PHASE4_PACKAGE"; then
  fail "package claims acceptance"
fi
if grep -Fq "3.9 passed" "$PHASE4_PACKAGE"; then
  fail "package claims the unrun 3.9 gate passed"
fi
if grep -Eq 'BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY|ghp_[A-Za-z0-9]{20,}' "$PHASE4_PACKAGE"; then
  fail "package appears to contain secret material"
fi

echo "Phase 4 review package: PASS"
