import { openSync, writeSync, closeSync, writeFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';

// Each process receives only its own credential over IPC. Credentials never
// appear in argv, persisted manifests, requests in evidence, or error messages.
const [agentId, logPath, summaryPath] = process.argv.slice(2);
const fd = openSync(logPath, 'wx', 0o600);
const log = (row) => writeSync(fd, `${JSON.stringify(row)}\n`);
let runId;
let credential;
let plan;
let active;
let watchdog;
const summaries = [];
const key = (seq) => ({ runId, round: plan.round, agentId, pid: process.pid,
  seq, pingId: `${runId}/${plan.round}/${agentId}/${seq}` });

function fatal(code) {
  clearTimeout(watchdog);
  log({ event: 'fatal', runId, agentId, pid: process.pid, code });
  closeSync(fd);
  process.exit(1);
}

function finishRound() {
  if (!active || active.sent !== plan.count || active.received !== plan.count
    || active.pendingCallbacks !== 0) return;
  clearTimeout(watchdog);
  const summary = { event: 'round_complete', runId, round: plan.round,
    agentId, pid: process.pid, sent: active.sent, received: active.received,
    backpressure: active.backpressure, maxOutstanding: active.maxOutstanding,
    sendSpanMs: active.lastSend - active.firstSend,
    elapsedMs: performance.now() - active.start,
    codes: active.codes, maxScheduleLagMs: active.maxScheduleLagMs };
  log(summary);
  summaries.push(summary);
  active = null;
  process.send({ type: 'round_complete', round: plan.round, summary });
}

function sendOne(seq) {
  const sent = performance.now();
  const fields = key(seq);
  const intent = { agentId, action: plan.action, target: plan.target };
  active.pending.set(fields.pingId, { fields, sent });
  active.sent += 1;
  active.firstSend ??= sent;
  active.lastSend = sent;
  active.maxOutstanding = Math.max(active.maxOutstanding, active.pending.size);
  active.maxScheduleLagMs = Math.max(active.maxScheduleLagMs,
    sent - active.start - seq * plan.intervalMs);
  log({ event: 'send', ...fields, intent,
    scheduledMs: seq * plan.intervalMs, sentMs: sent - active.start });
  active.pendingCallbacks += 1;
  const writable = process.send({ type: 'ping', ...fields, request: { credential, intent } }, (error) => {
    if (error) {
      log({ event: 'send_error', ...fields, code: 'IPC_SEND_FAILED' });
      fatal('IPC_SEND_FAILED');
    }
    active.pendingCallbacks -= 1;
    finishRound();
  });
  if (!writable) active.backpressure += 1;
}

// Absolute per-round schedule: response latency does not throttle offered load.
function pump() {
  if (!active) return;
  while (active.sent < plan.count) {
    const wait = active.start + active.sent * plan.intervalMs - performance.now();
    if (wait > 0) {
      setTimeout(pump, Math.max(1, Math.ceil(wait)));
      return;
    }
    sendOne(active.sent);
  }
}

process.on('message', (message) => {
  if (message.type === 'init') {
    if (credential) return fatal('DUPLICATE_INIT');
    runId = message.runId;
    credential = message.credential;
    log({ event: 'ready', runId, agentId, pid: process.pid });
    process.send({ type: 'ready', agentId, pid: process.pid });
  } else if (message.type === 'prepare') {
    if (!credential || active) return fatal('INVALID_PREPARE');
    plan = message.plan;
    process.send({ type: 'prepared', round: plan.round });
  } else if (message.type === 'go') {
    if (!plan || active || message.round !== plan.round) return fatal('INVALID_START');
    active = { start: performance.now(), pending: new Map(), sent: 0, received: 0,
      pendingCallbacks: 0, backpressure: 0, maxOutstanding: 0,
      firstSend: null, lastSend: null, maxScheduleLagMs: 0, codes: {} };
    watchdog = setTimeout(() => {
      for (const { fields } of active.pending.values()) log({ event: 'timeout', ...fields });
      fatal('ROUND_TIMEOUT');
    }, 30_000);
    pump();
  } else if (message.type === 'response') {
    const pending = active?.pending.get(message.pingId);
    if (!pending || message.round !== plan.round) return fatal('UNEXPECTED_RESPONSE');
    log({ event: 'response', ...pending.fields, result: message.result,
      rttMs: performance.now() - pending.sent });
    active.pending.delete(message.pingId);
    active.received += 1;
    active.codes[message.result.code] = (active.codes[message.result.code] ?? 0) + 1;
    finishRound();
  } else if (message.type === 'stop') {
    if (active || summaries.length !== message.expectedRounds) return fatal('INCOMPLETE_STOP');
    log({ event: 'done', runId, agentId, pid: process.pid, rounds: summaries.length });
    writeFileSync(summaryPath, `${JSON.stringify({ runId, agentId, pid: process.pid,
      rounds: summaries }, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
    closeSync(fd);
    process.disconnect();
  } else fatal('UNKNOWN_MESSAGE');
});

process.on('disconnect', () => {
  if (active || summaries.length === 0) process.exitCode = 1;
});
process.on('uncaughtException', () => fatal('WORKER_EXCEPTION'));
process.on('unhandledRejection', () => fatal('WORKER_REJECTION'));
