/**
 * Northstar RFC 8785 JCS profile for TL-PX 0.2 security envelopes.
 *
 * This is not the 0.1 audit hasher (`canonicalJson` in audit.mjs).
 * 0.1 seals stay on that function. 0.2 hashes MUST use this profile.
 */

export const JCS_MAX_SAFE_INTEGER = Number.MAX_SAFE_INTEGER;
export const JCS_MIN_SAFE_INTEGER = Number.MIN_SAFE_INTEGER;

function jcsError(message) {
  return new Error(`jcs: ${message}`);
}

function skipWs(p) {
  const s = p.s;
  while (p.i < s.length) {
    const c = s.charCodeAt(p.i);
    if (c === 0x20 || c === 0x0a || c === 0x0d || c === 0x09) p.i++;
    else break;
  }
}

function peek(p) {
  return p.s[p.i];
}

function parseValue(p) {
  skipWs(p);
  const c = peek(p);
  if (c === undefined) throw jcsError("unexpected end");
  if (c === "{") return parseObject(p);
  if (c === "[") return parseArray(p);
  if (c === '"') return parseString(p);
  if (c === "t" || c === "f" || c === "n") return parseLiteral(p);
  if (c === "-" || (c >= "0" && c <= "9")) return parseNumber(p);
  throw jcsError(`unexpected character ${JSON.stringify(c)}`);
}

function parseLiteral(p) {
  if (p.s.startsWith("true", p.i)) {
    p.i += 4;
    return true;
  }
  if (p.s.startsWith("false", p.i)) {
    p.i += 5;
    return false;
  }
  if (p.s.startsWith("null", p.i)) {
    p.i += 4;
    return null;
  }
  throw jcsError("invalid literal");
}

function parseNumber(p) {
  const start = p.i;
  if (p.s[p.i] === "-") p.i++;
  if (p.s[p.i] === "0") {
    p.i++;
  } else if (p.s[p.i] >= "1" && p.s[p.i] <= "9") {
    while (p.s[p.i] >= "0" && p.s[p.i] <= "9") p.i++;
  } else {
    throw jcsError("invalid number");
  }
  const next = p.s[p.i];
  if (next === "." || next === "e" || next === "E") {
    throw jcsError("floating-point values are prohibited");
  }
  const token = p.s.slice(start, p.i);
  if (token === "-0") throw jcsError("negative zero is prohibited");
  let n;
  try {
    n = Number(token);
  } catch {
    throw jcsError(`invalid integer ${token}`);
  }
  if (!Number.isSafeInteger(n)) {
    throw jcsError(`integer out of safe range: ${token}`);
  }
  return n;
}

function parseHexEscape(p) {
  const hex = p.s.slice(p.i, p.i + 4);
  if (!/^[0-9a-fA-F]{4}$/.test(hex)) throw jcsError("bad unicode escape");
  p.i += 4;
  return parseInt(hex, 16);
}

function parseString(p) {
  if (p.s[p.i] !== '"') throw jcsError("expected string");
  p.i++;
  let out = "";
  while (p.i < p.s.length) {
    const cu = p.s.charCodeAt(p.i);
    if (p.s[p.i] === '"') {
      p.i++;
      return out;
    }
    if (p.s[p.i] === "\\") {
      p.i++;
      const e = p.s[p.i];
      p.i++;
      if (e === '"' || e === "\\" || e === "/") out += e;
      else if (e === "b") out += "\b";
      else if (e === "f") out += "\f";
      else if (e === "n") out += "\n";
      else if (e === "r") out += "\r";
      else if (e === "t") out += "\t";
      else if (e === "u") {
        const unit = parseHexEscape(p);
        if (unit >= 0xd800 && unit <= 0xdbff) {
          if (p.s[p.i] !== "\\" || p.s[p.i + 1] !== "u") {
            throw jcsError("lone surrogate");
          }
          p.i += 2;
          const low = parseHexEscape(p);
          if (low < 0xdc00 || low > 0xdfff) throw jcsError("lone surrogate");
          out += String.fromCharCode(unit, low);
        } else if (unit >= 0xdc00 && unit <= 0xdfff) {
          throw jcsError("lone surrogate");
        } else {
          out += String.fromCharCode(unit);
        }
      } else {
        throw jcsError("bad escape");
      }
      continue;
    }
    if (cu < 0x20) throw jcsError("unescaped control character");
    if (cu >= 0xd800 && cu <= 0xdbff) {
      const low = p.s.charCodeAt(p.i + 1);
      if (!(low >= 0xdc00 && low <= 0xdfff)) throw jcsError("lone surrogate");
      out += p.s[p.i] + p.s[p.i + 1];
      p.i += 2;
      continue;
    }
    if (cu >= 0xdc00 && cu <= 0xdfff) throw jcsError("lone surrogate");
    out += p.s[p.i];
    p.i++;
  }
  throw jcsError("unterminated string");
}

function parseObject(p) {
  p.i++; // {
  skipWs(p);
  const obj = Object.create(null);
  const seen = new Set();
  if (peek(p) === "}") {
    p.i++;
    return obj;
  }
  while (true) {
    skipWs(p);
    if (peek(p) !== '"') throw jcsError("expected object key");
    const key = parseString(p);
    if (seen.has(key)) throw jcsError(`duplicate key ${JSON.stringify(key)}`);
    seen.add(key);
    skipWs(p);
    if (peek(p) !== ":") throw jcsError("expected :");
    p.i++;
    obj[key] = parseValue(p);
    skipWs(p);
    const c = peek(p);
    if (c === ",") {
      p.i++;
      continue;
    }
    if (c === "}") {
      p.i++;
      return obj;
    }
    throw jcsError("expected , or }");
  }
}

function parseArray(p) {
  p.i++; // [
  skipWs(p);
  const arr = [];
  if (peek(p) === "]") {
    p.i++;
    return arr;
  }
  while (true) {
    arr.push(parseValue(p));
    skipWs(p);
    const c = peek(p);
    if (c === ",") {
      p.i++;
      continue;
    }
    if (c === "]") {
      p.i++;
      return arr;
    }
    throw jcsError("expected , or ]");
  }
}

/**
 * Parse JSON text under the Northstar JCS profile.
 * Rejects duplicate keys, floats, and unsafe integers.
 */
export function parseRestrictedJson(text) {
  if (typeof text !== "string") throw jcsError("JSON text must be a string");
  const p = { s: text, i: 0 };
  const value = parseValue(p);
  skipWs(p);
  if (p.i !== p.s.length) throw jcsError("trailing data");
  return value;
}

/** RFC 8785 §3.2.3: unsigned UTF-16 code units, not Unicode code points. */
function compareUtf16(a, b) {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const d = a.charCodeAt(i) - b.charCodeAt(i);
    if (d) return d;
  }
  return a.length - b.length;
}

function assertValidUtf16(str, label = "string") {
  for (let i = 0; i < str.length; i++) {
    const cu = str.charCodeAt(i);
    if (cu >= 0xd800 && cu <= 0xdbff) {
      const low = str.charCodeAt(i + 1);
      if (!(low >= 0xdc00 && low <= 0xdfff)) {
        throw jcsError(`lone surrogate in ${label}`);
      }
      i++;
      continue;
    }
    if (cu >= 0xdc00 && cu <= 0xdfff) {
      throw jcsError(`lone surrogate in ${label}`);
    }
  }
}

function encodeString(str) {
  assertValidUtf16(str);
  let out = '"';
  for (let i = 0; i < str.length; i++) {
    const cu = str.charCodeAt(i);
    if (cu === 0x22) out += '\\"';
    else if (cu === 0x5c) out += "\\\\";
    else if (cu === 0x08) out += "\\b";
    else if (cu === 0x09) out += "\\t";
    else if (cu === 0x0a) out += "\\n";
    else if (cu === 0x0c) out += "\\f";
    else if (cu === 0x0d) out += "\\r";
    else if (cu < 0x20) out += `\\u${cu.toString(16).padStart(4, "0")}`;
    else if (cu >= 0xd800 && cu <= 0xdbff) {
      out += str[i] + str[i + 1];
      i++;
    } else {
      out += str[i];
    }
  }
  return `${out}"`;
}

function assertPlainObject(value) {
  const proto = Object.getPrototypeOf(value);
  if (proto !== Object.prototype && proto !== null) {
    throw jcsError("only plain objects may be canonicalized");
  }
}

/**
 * Serialize a JS value to RFC 8785 JCS under the Northstar profile.
 */
export function canonicalize(value) {
  if (value === null) return "null";
  if (value === true) return "true";
  if (value === false) return "false";
  if (typeof value === "string") return encodeString(value);
  if (typeof value === "number") {
    if (Object.is(value, -0)) throw jcsError("negative zero is prohibited");
    if (!Number.isSafeInteger(value)) {
      throw jcsError("floating-point and unsafe integers are prohibited");
    }
    return String(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((v) => canonicalize(v)).join(",")}]`;
  }
  if (value && typeof value === "object") {
    assertPlainObject(value);
    const keys = Object.keys(value).sort(compareUtf16);
    const seen = new Set();
    const parts = [];
    for (const key of keys) {
      if (seen.has(key)) throw jcsError(`duplicate key ${JSON.stringify(key)}`);
      seen.add(key);
      const v = value[key];
      if (v === undefined) throw jcsError(`undefined value for key ${JSON.stringify(key)}`);
      parts.push(`${encodeString(key)}:${canonicalize(v)}`);
    }
    return `{${parts.join(",")}}`;
  }
  throw jcsError(`unsupported type ${typeof value}`);
}

export function canonicalizeJsonText(text) {
  return canonicalize(parseRestrictedJson(text));
}

export function utf8Bytes(canonical) {
  return Buffer.from(canonical, "utf8");
}

export function utf8Hex(canonical) {
  return utf8Bytes(canonical).toString("hex");
}
