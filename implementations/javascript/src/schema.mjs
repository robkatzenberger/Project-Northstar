/**
 * Tiny JSON Schema subset for TL-PX 0.2.
 * Supports: type, const, enum, required, properties, additionalProperties,
 * minLength, pattern, minimum, items, anyOf, $ref.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const schemaDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../../schemas/tlpx-0.2"
);

const registry = new Map();

function loadSchemaFile(fileName) {
  if (registry.has(fileName)) return registry.get(fileName);
  const full = path.join(schemaDir, fileName);
  const schema = JSON.parse(fs.readFileSync(full, "utf8"));
  registry.set(fileName, schema);
  if (schema.$id) registry.set(schema.$id, schema);
  return schema;
}

export function loadTlpx02Schemas() {
  for (const name of fs.readdirSync(schemaDir)) {
    if (name.endsWith(".schema.json")) loadSchemaFile(name);
  }
  return registry;
}

function resolveRef(ref, current, root) {
  if (ref.startsWith("#/")) {
    return getPath(root, ref.slice(2).split("/"));
  }
  const [file, frag] = ref.split("#");
  const target = loadSchemaFile(path.basename(file));
  if (!frag) return target;
  return getPath(target, frag.replace(/^\//, "").split("/"));
}

function getPath(obj, parts) {
  let cur = obj;
  for (const p of parts) {
    if (cur == null || typeof cur !== "object") {
      throw new Error(`schema $ref missing ${parts.join("/")}`);
    }
    cur = cur[p];
  }
  return cur;
}

function typeOf(value) {
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  if (Number.isInteger(value)) return "integer";
  return typeof value;
}

export function validateAgainst(schema, value, root = schema) {
  const errors = [];
  walk(schema, value, root, "$", errors);
  return { ok: errors.length === 0, errors };
}

function walk(schema, value, root, path, errors) {
  if (!schema || typeof schema !== "object") return;
  if (schema.$ref) {
    walk(resolveRef(schema.$ref, schema, root), value, root, path, errors);
    return;
  }
  if (schema.anyOf) {
    const ok = schema.anyOf.some((sub) => {
      const inner = [];
      walk(sub, value, root, path, inner);
      return inner.length === 0;
    });
    if (!ok) errors.push(`${path}: does not match anyOf`);
    return;
  }
  if (schema.const !== undefined && value !== schema.const) {
    errors.push(`${path}: must be ${JSON.stringify(schema.const)}`);
  }
  if (schema.enum && !schema.enum.includes(value)) {
    errors.push(`${path}: must be one of ${schema.enum.join("|")}`);
  }
  if (schema.type) {
    const allowed = Array.isArray(schema.type) ? schema.type : [schema.type];
    const t = typeOf(value);
    const matches = allowed.some((a) => a === t || (a === "number" && (t === "integer" || t === "number")));
    if (!matches) errors.push(`${path}: type ${t} is not ${allowed.join("|")}`);
  }
  if (typeof value === "string") {
    if (schema.minLength != null && value.length < schema.minLength) {
      errors.push(`${path}: shorter than ${schema.minLength}`);
    }
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) {
      errors.push(`${path}: does not match ${schema.pattern}`);
    }
  }
  if (typeof value === "number" && schema.minimum != null && value < schema.minimum) {
    errors.push(`${path}: below minimum ${schema.minimum}`);
  }
  if (Array.isArray(value) && schema.items) {
    value.forEach((item, i) => walk(schema.items, item, root, `${path}[${i}]`, errors));
  }
  if (value && typeof value === "object" && !Array.isArray(value) && schema.properties) {
    if (Array.isArray(schema.required)) {
      for (const key of schema.required) {
        if (!(key in value)) errors.push(`${path}.${key}: required`);
      }
    }
    if (schema.additionalProperties === false) {
      for (const key of Object.keys(value)) {
        if (!schema.properties[key]) errors.push(`${path}.${key}: additional property`);
      }
    }
    for (const [key, sub] of Object.entries(schema.properties)) {
      if (key in value) walk(sub, value[key], root, `${path}.${key}`, errors);
    }
  }
}

export function validateSchemaFile(fileName, value) {
  const schema = loadSchemaFile(fileName);
  return validateAgainst(schema, value, schema);
}
