#!/usr/bin/env node
import { constants } from 'node:fs';
import { open } from 'node:fs/promises';
import { submitIntent, observeAssessment } from '../src/agent-client.js';

function fault(code, message) { return Object.assign(new Error(message), { name: 'AgentClientError', code }); }

async function credentialFromFile(path) {
  if (!path) throw fault('CREDENTIAL_FILE_REQUIRED', 'Set APEX_CREDENTIAL_FILE to a private agent credential file.');
  let handle;
  try {
    handle = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW);
    const stat = await handle.stat();
    if (!stat.isFile() || (stat.mode & 0o077) !== 0 || stat.size > 128
      || (typeof process.getuid === 'function' && stat.uid !== process.getuid())) {
      throw fault('UNSAFE_CREDENTIAL_FILE', 'The credential file must be private and owned by the current user.');
    }
    const token = (await handle.readFile('utf8')).trim();
    if (!/^[a-f0-9]{64}$/.test(token)) throw fault('INVALID_CREDENTIAL', 'The credential file is invalid.');
    return token;
  } catch (error) {
    if (error.name === 'AgentClientError') throw error;
    throw fault('CREDENTIAL_FILE_UNAVAILABLE', 'Could not read the private credential file.');
  } finally { await handle?.close(); }
}

async function readIntent() {
  let size = 0;
  const chunks = [];
  for await (const chunk of process.stdin) {
    size += chunk.length;
    if (size > 8192) throw fault('INTENT_TOO_LARGE', 'Intent input exceeds 8 KiB.');
    chunks.push(chunk);
  }
  try { return JSON.parse(Buffer.concat(chunks).toString('utf8')); }
  catch { throw fault('INVALID_INTENT', 'Provide one intent JSON object on standard input.'); }
}

try {
  const [command, assessmentId, ...extra] = process.argv.slice(2);
  if (!['submit', 'status'].includes(command) || extra.length || (command === 'submit' && assessmentId)
    || (command === 'status' && !assessmentId)) {
    throw fault('USAGE', 'Use: node bin/apex-agent.mjs submit < intent.json; or status <assessment-id>.');
  }
  const agentId = process.env.APEX_AGENT_ID;
  if (!agentId || agentId !== agentId.trim() || !/^[A-Za-z][A-Za-z0-9._:-]{0,63}$/.test(agentId)) {
    throw fault('AGENT_ID_REQUIRED', 'Set APEX_AGENT_ID to the enrolled agent identity.');
  }
  const credential = await credentialFromFile(process.env.APEX_CREDENTIAL_FILE);
  const url = process.env.APEX_URL || 'http://127.0.0.1:43129';
  if (command === 'submit') {
    const intent = await readIntent();
    if (intent?.agentId !== agentId) throw fault('IDENTITY_MISMATCH', 'The intent must name APEX_AGENT_ID.');
    const result = await submitIntent({ url, credential, intent });
    process.stdout.write(`${JSON.stringify(result)}\n`);
    process.exitCode = { APPROVE: 0, ESCALATE: 2, BLOCKED: 3, ERROR: 1 }[result.status];
  } else {
    const record = await observeAssessment({ url, credential, assessmentId });
    if (record.assessment.request.agentId !== agentId) throw fault('IDENTITY_MISMATCH', 'The saved assessment belongs to a different identity.');
    let status = record.review?.decision ?? record.assessment.result.status;
    let code = record.review ? 'HUMAN_DECISION_RECORDED' : record.assessment.result.code;
    if (status !== 'DENY' && ['APPROVE', 'ESCALATE'].includes(status) && !record.current) {
      status = 'REASSESSMENT_REQUIRED';
      code = 'ASSESSMENT_STALE';
    }
    process.stdout.write(`${JSON.stringify({ ...record, client: { status, code, execution: 'NOT_EXECUTED' } })}\n`);
    process.exitCode = { APPROVE: 0, ESCALATE: 2, BLOCKED: 3, DENY: 3, REASSESSMENT_REQUIRED: 3, ERROR: 1 }[status];
  }
} catch (error) {
  const safe = error?.name === 'AgentClientError';
  process.stderr.write(`${JSON.stringify({ status: 'ERROR', code: safe ? error.code : 'CLIENT_ERROR',
    error: safe ? error.message : 'The agent client could not complete the request.', execution: 'NOT_EXECUTED' })}\n`);
  process.exitCode = 1;
}
