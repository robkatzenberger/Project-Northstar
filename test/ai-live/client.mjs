import { appendFileSync, readFileSync, writeFileSync, renameSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import { setTimeout } from 'node:timers/promises';

// Invoked by an AI participant. This transport attaches its assigned credential
// and records observations; it does not create intent or evaluate policy.
const [mailbox, participantId] = process.argv.slice(2);
if (!mailbox?.startsWith('/private/tmp/apex-ai-live-')
  || !['agent.ai.cedar', 'agent.ai.maple'].includes(participantId)) throw new Error('Invalid client invocation');
const { runId, evidence } = JSON.parse(readFileSync(join(mailbox, 'connection.json'), 'utf8'));
const input = JSON.parse(readFileSync(0, 'utf8'));
if (!/^[a-z0-9-]{1,80}$/.test(input.requestId) || ![1, 2].includes(input.round)
  || !['own', 'missing'].includes(input.credentialMode) || !Object.hasOwn(input, 'intent')) {
  throw new Error('Expected requestId, round, credentialMode (own/missing), and intent');
}
const { token } = JSON.parse(readFileSync(join(mailbox, 'credentials', `${participantId}.json`), 'utf8'));
const meta = { runId, round: input.round, requestId: input.requestId, participantId,
  intent: input.intent, credentialMode: input.credentialMode };
const log = (row) => appendFileSync(join(evidence, 'agents', `${participantId}.jsonl`), `${JSON.stringify(row)}\n`, { mode: 0o600 });
const start = performance.now();
log({ event: 'submitted', ...meta, at: new Date().toISOString() });
const staging = join(mailbox, 'requests', `${input.requestId}.next`);
const destination = join(mailbox, 'requests', `${input.requestId}.json`);
if (existsSync(destination) || existsSync(join(mailbox, 'responses', `${input.requestId}.json`))) throw new Error('Request ID already used');
writeFileSync(staging, JSON.stringify({ ...meta, request: {
  credential: input.credentialMode === 'own' ? token : '', intent: input.intent } }), { flag: 'wx', mode: 0o600 });
renameSync(staging, destination);
const responsePath = join(mailbox, 'responses', `${input.requestId}.json`);
while (!existsSync(responsePath)) {
  if (performance.now() - start > 20_000) {
    log({ event: 'timeout', ...meta });
    throw new Error('Gate response timeout');
  }
  await setTimeout(20);
}
const receipt = JSON.parse(readFileSync(responsePath, 'utf8'));
if (receipt.runId !== runId || receipt.requestId !== input.requestId || receipt.participantId !== participantId
  || receipt.round !== input.round) throw new Error('Response correlation mismatch');
log({ event: 'observed', ...receipt, rttMs: performance.now() - start, at: new Date().toISOString() });
process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
