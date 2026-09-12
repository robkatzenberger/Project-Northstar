const fs = require("node:fs");

function parseScalar(value) {
  if (value === "true") {
    return true;
  }

  if (value === "false") {
    return false;
  }

  if (/^-?\d+$/.test(value)) {
    return Number(value);
  }

  const quoted = value.match(/^"(.*)"$/);
  if (quoted) {
    return quoted[1];
  }

  return value;
}

function readPolicy(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const lines = text.split(/\r?\n/);
  const rules = [];
  let currentRule = null;

  for (const rawLine of lines) {
    const line = rawLine.trimEnd();
    const trimmed = line.trim();

    if (!trimmed || trimmed.startsWith("#") || trimmed === "rules:") {
      continue;
    }

    const ruleMatch = line.match(/^\s*-\s+id:\s+(.+)$/);
    if (ruleMatch) {
      currentRule = { id: parseScalar(ruleMatch[1].trim()) };
      rules.push(currentRule);
      continue;
    }

    const fieldMatch = line.match(/^\s+([A-Za-z_]+):\s+(.+)$/);
    if (fieldMatch && currentRule) {
      currentRule[fieldMatch[1]] = parseScalar(fieldMatch[2].trim());
      continue;
    }

    throw new Error(`Invalid policy line: ${rawLine}`);
  }

  return { rules };
}

module.exports = {
  parseScalar,
  readPolicy
};
