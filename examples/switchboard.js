import { randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createSwitchboard, createTokenAuthenticator } from '../src/switchboard.js';

const registry = JSON.parse(readFileSync(new URL('../config/agents.json', import.meta.url), 'utf8'));
// Synthetic credentials exist only for this process and are never printed.
const token = randomBytes(32).toString('hex');
const authenticate = createTokenAuthenticator([{ principalId: 'report-assistant', token }]);
const switchboard = createSwitchboard(registry, { authenticate });
const intent = { agentId: 'report-assistant', action: 'report.read', target: 'report:demo' };

for (const [example, request] of [
  ['Whitelisted scope', { credential: token, intent }],
  ['Target outside scope', { credential: token, intent: { ...intent, target: 'report:private' } }],
  ['Claimed identity substitution', { credential: token, intent: { ...intent, agentId: 'paused-assistant' } }],
  ['Missing credential', { credential: null, intent }],
]) {
  console.log(JSON.stringify({ example, ...switchboard.screen(request) }, null, 2));
}
