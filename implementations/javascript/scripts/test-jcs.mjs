/**
 * TL-PX 0.2 JCS / domain-hash golden fixtures (slice 2.2).
 * Distinct from the frozen TL-PX 0.1 conformance suite.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  canonicalize,
  canonicalizeJsonText,
  parseRestrictedJson,
  utf8Hex
} from "../src/jcs.mjs";
import {
  HASH_PATTERN,
  assertHashString,
  digestHex,
  hashString,
  hashValue
} from "../src/hash.mjs";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const goldenPath = path.join(repo, "tests/fixtures/tlpx-0.2/jcs/golden.json");
const golden = JSON.parse(fs.readFileSync(goldenPath, "utf8"));

let n = 0;
function assert(c, m) {
  if (!c) throw new Error(m);
  n++;
  console.log(`  ok  ${m}`);
}

function throws(fn, snippet, msg) {
  let err;
  try {
    fn();
  } catch (e) {
    err = e;
  }
  assert(err instanceof Error, `${msg}: threw`);
  if (snippet) {
    assert(String(err.message).includes(snippet), `${msg}: ${JSON.stringify(snippet)}`);
  }
}

console.log("TL-PX 0.2 JCS / hash fixtures\n");

{
  console.log("fixture file");
  assert(golden.profile === "northstar-jcs-v1", "profile id");
  assert(golden.standard_version === "0.2.0", "0.2 version");
  assert(Array.isArray(golden.accept) && golden.accept.length > 0, "accept vectors");
  assert(Array.isArray(golden.reject) && golden.reject.length > 0, "reject vectors");
}

{
  console.log("accept vectors");
  const nfc = golden.accept.find((r) => r.id === "unicode-nfc");
  const esc = golden.accept.find((r) => r.id === "unicode-escape-nfc");
  const nfd = golden.accept.find((r) => r.id === "nfd-key");
  const astral = golden.accept.find((r) => r.id === "utf16-sort-astral-before-bmp");
  assert(nfc && esc && nfc.canonical === esc.canonical, "NFC and \\u00e9 match");
  assert(nfd && nfd.canonical !== nfc.canonical, "NFD key is distinct");
  assert(astral, "astral/BMP sort vector present");
  assert(
    astral.canonical.charCodeAt(2) === 0xd800,
    "U+10000 (lead surrogate 0xD800) sorts before U+E000"
  );

  for (const row of golden.accept) {
    const canonical = canonicalizeJsonText(row.input_json);
    assert(canonical === row.canonical, `${row.id} canonical`);
    assert(utf8Hex(canonical) === row.canonical_utf8_hex, `${row.id} utf8 hex`);
    for (const [domain, expected] of Object.entries(row.sha256)) {
      assert(HASH_PATTERN.test(expected), `${row.id} ${domain} pattern`);
      assert(hashString(domain, canonical) === expected, `${row.id} ${domain} hash`);
      assert(row.digest_hex?.[domain] === expected.slice(7), `${row.id} ${domain} digest_hex`);
      assert(digestHex(domain, canonical) === row.digest_hex[domain], `${row.id} ${domain} raw digest`);
    }
  }
}

{
  console.log("reject vectors");
  for (const row of golden.reject) {
    throws(() => canonicalizeJsonText(row.input_json), row.error_contains, row.id);
  }
}

{
  console.log("JS value path");
  assert(canonicalize({ b: 1, a: 2 }) === '{"a":2,"b":1}', "object key sort");
  assert(canonicalize(null) === "null", "null value");
  throws(() => canonicalize(1.5), "floating-point", "JS float rejected");
  throws(() => canonicalize(-0), "negative zero", "JS -0 rejected");
  throws(() => canonicalize(9007199254740992), "unsafe", "JS unsafe integer rejected");
  throws(() => parseRestrictedJson('{"a":1,"a":2}'), "duplicate key", "parse duplicate");
  const astralFirst = canonicalize({ "\uE000": 1, "\u{10000}": 2 });
  assert(astralFirst.charCodeAt(2) === 0xd800, "JS value UTF-16 sort");
  throws(() => canonicalize({ "\uD800": 1 }), "lone surrogate", "JS lone high surrogate key");
  throws(() => canonicalize({ a: "\uDC00" }), "lone surrogate", "JS lone low surrogate value");
}

{
  console.log("hash representation");
  const h = hashValue("intent", {});
  assertHashString(h);
  throws(() => assertHashString("SHA256:" + "a".repeat(64)), "expected sha256", "uppercase rejected");
  throws(() => assertHashString("sha256:" + "A".repeat(64)), "expected sha256", "uppercase hex rejected");
  throws(() => assertHashString("0x" + "a".repeat(64)), "expected sha256", "0x rejected");
  throws(() => assertHashString("sha256:" + "a".repeat(63)), "expected sha256", "truncated rejected");
}

{
  console.log("must not use default JSON.stringify");
  const obj = { b: 1, a: 2 };
  assert(JSON.stringify(obj) === '{"b":1,"a":2}', "default stringify preserves insert order");
  assert(canonicalize(obj) === '{"a":2,"b":1}', "JCS reorders keys");
}

console.log(`\nJCS tests passed (${n}).`);
