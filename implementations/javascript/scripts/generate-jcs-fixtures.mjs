/**
 * Emit language-neutral TL-PX 0.2 JCS golden fixtures.
 * Used to author tests/fixtures/tlpx-0.2/jcs/golden.json.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  canonicalizeJsonText,
  utf8Hex
} from "../src/jcs.mjs";
import { HASH_DOMAINS, digestHex, hashString } from "../src/hash.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const outPath = path.join(root, "tests/fixtures/tlpx-0.2/jcs/golden.json");

const domains = ["intent", "authorized-action", "executed-action", "approval-context"];

const acceptInputs = [
  { id: "empty-object", input_json: "{}" },
  { id: "empty-array", input_json: "[]" },
  { id: "null", input_json: "null" },
  { id: "true", input_json: "true" },
  { id: "false", input_json: "false" },
  { id: "empty-string", input_json: '""' },
  { id: "integer-zero", input_json: "0" },
  { id: "integer-negative", input_json: "-42" },
  { id: "integer-max-safe", input_json: "9007199254740991" },
  { id: "integer-min-safe", input_json: "-9007199254740991" },
  { id: "key-order", input_json: '{"b":1,"a":2}' },
  { id: "null-value", input_json: '{"a":null}' },
  { id: "nested", input_json: '{"a":{"c":1,"b":2},"d":[3,1]}' },
  { id: "unicode-nfc", input_json: '{"café":1}' },
  { id: "unicode-escape-nfc", input_json: '{"caf\\u00e9":1}' },
  { id: "nfd-key", input_json: '{"e\\u0301":1}' },
  { id: "string-escapes", input_json: '{"x":"a\\"b\\\\c\\n"}' },
  { id: "control-u0001", input_json: '{"x":"\\u0001"}' },
  {
    id: "approval-context",
    input_json:
      '{"authorized_action_hash":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","policy_bundle_hash":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","approval_route":["human.ops.alex"],"material_display_fields":["action","target"],"renderer_id":"glass.approval.v1","renderer_version":"1.0.0"}'
  },
  {
    id: "utf16-sort-astral-before-bmp",
    input_json: '{"\\uE000":1,"\\uD800\\uDC00":2}'
  }
];

const rejectInputs = [
  { id: "duplicate-key", input_json: '{"a":1,"a":2}', error_contains: "duplicate key" },
  { id: "float-dot", input_json: "1.5", error_contains: "floating-point" },
  { id: "float-integer-dot", input_json: "1.0", error_contains: "floating-point" },
  { id: "scientific", input_json: "1e2", error_contains: "floating-point" },
  { id: "negative-zero", input_json: "-0", error_contains: "negative zero" },
  { id: "unsafe-integer", input_json: "9007199254740992", error_contains: "safe range" },
  { id: "trailing-data", input_json: "{}{}", error_contains: "trailing" },
  { id: "lone-high-surrogate-key", input_json: '{"\\uD800":1}', error_contains: "lone surrogate" },
  { id: "lone-low-surrogate-value", input_json: '{"a":"\\uDC00"}', error_contains: "lone surrogate" },
  { id: "lone-high-then-non-low", input_json: '{"\\uD800\\u0020":1}', error_contains: "lone surrogate" }
];

const accept = acceptInputs.map((row) => {
  const canonical = canonicalizeJsonText(row.input_json);
  const sha256 = {};
  const digest_hex = {};
  for (const d of domains) {
    digest_hex[d] = digestHex(d, canonical);
    sha256[d] = hashString(d, canonical);
  }
  return {
    ...row,
    canonical,
    canonical_utf8_hex: utf8Hex(canonical),
    digest_hex,
    sha256
  };
});

const fixtures = {
  profile: "northstar-jcs-v1",
  standard: "TL-PX",
  standard_version: "0.2.0",
  hash_pattern: "^sha256:[0-9a-f]{64}$",
  domains: Object.fromEntries(
    Object.entries(HASH_DOMAINS).map(([k, v]) => [k, Array.from(Buffer.from(v, "utf8"))])
  ),
  notes: [
    "Domain prefixes are UTF-8 bytes including a trailing NUL (0x00).",
    "canonical is the exact JCS Unicode string; canonical_utf8_hex is its UTF-8 encoding.",
    "digest_hex[domain] is the raw 32-byte SHA-256 as 64 lowercase hex.",
    "sha256[domain] is exactly 'sha256:' + digest_hex[domain].",
    "Do not hash JSON.stringify output. 0.1 audit canonicalJson is a different function.",
    "unicode-nfc and unicode-escape-nfc MUST produce identical canonical bytes.",
    "nfd-key is not the same key as café (U+00E9)."
  ],
  accept,
  reject: rejectInputs
};

fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(fixtures, null, 2)}\n`);
console.log(`wrote ${outPath}`);
console.log(`accept=${accept.length} reject=${rejectInputs.length}`);
for (const row of accept) {
  console.log(`${row.id} ${row.canonical} ${row.sha256.intent}`);
}
