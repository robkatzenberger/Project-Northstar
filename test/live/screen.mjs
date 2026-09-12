import { readFileSync, appendFileSync } from 'node:fs';
import { createSwitchboard, createTokenAuthenticator } from '../../src/switchboard.js';

const round = process.argv[2];
const requestPath = process.argv[3];
if (!round || !requestPath) {
  console.error('usage: node screen.mjs <round-a|round-b> <request.json>');
  process.exit(2);
}

const registry = JSON.parse(readFileSync(new URL(`./registry-${round}.json`, import.meta.url)));
const tokens = JSON.parse(readFileSync(new URL('./credentials.json', import.meta.url)));
const request = JSON.parse(readFileSync(requestPath, 'utf8'));

const authenticate = createTokenAuthenticator(
  Object.entries(tokens).map(([principalId, token]) => ({ principalId, token }))
);
const switchboard = createSwitchboard(registry, { authenticate });
const result = switchboard.screen(request);
const record = {
  at: new Date().toISOString(),
  round,
  request: {
    // never log raw credential
    credentialPresent: typeof request.credential === 'string' && request.credential.length > 0,
    credentialLen: typeof request.credential === 'string' ? request.credential.length : 0,
    intent: request.intent ?? null,
  },
  result,
};
appendFileSync(new URL('./switchboard-log.jsonl', import.meta.url), JSON.stringify(record) + '\n');
console.log(JSON.stringify(record, null, 2));
