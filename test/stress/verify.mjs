import { readFile, readdir, stat } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const ISSUE_LIMIT = 200;
const FILE_LIMIT = 64 * 1024 * 1024;
const agentPattern = /^[A-Za-z][A-Za-z0-9._:-]{0,63}$/;
const resultFields = ['stage', 'status', 'code', 'reason', 'registry', 'principalId', 'intent', 'grant', 'next'];
const object = (value) => value !== null && typeof value === 'object' && !Array.isArray(value);
const finite = (value) => typeof value === 'number' && Number.isFinite(value) && value >= 0;
const integer = (value) => Number.isSafeInteger(value) && value >= 0;
const fieldsEqual = (value, fields) => object(value)
  && isDeepStrictEqual(Object.keys(value).sort(), [...fields].sort());

function distribution(values) {
  const sorted = values.filter(finite).sort((a, b) => a - b);
  const percentile = (fraction) => sorted.length
    ? sorted[Math.max(0, Math.ceil(sorted.length * fraction) - 1)] : null;
  return { p50: percentile(0.50), p95: percentile(0.95), p99: percentile(0.99), max: sorted.at(-1) ?? null };
}

/** Independently reconciles plain JSONL evidence; imports no Switchboard runtime. */
export async function verifyRun(runDir) {
  const issues = [];
  let issueCount = 0;
  let expectedPings = 0;
  const summaries = [];
  const issue = (message) => {
    issueCount += 1;
    if (issues.length < ISSUE_LIMIT) issues.push(message);
  };
  const finish = () => ({ ok: issueCount === 0, issueCount, issues, expectedPings, rounds: summaries });

  try {
    if (typeof runDir !== 'string' || !runDir) {
      issue('A run directory is required.');
      return finish();
    }
    const directory = resolve(runDir);
    async function contents(relative) {
      try {
        const file = join(directory, relative);
        const info = await stat(file);
        if (!info.isFile() || info.size > FILE_LIMIT) {
          issue(`${relative}: expected a regular file no larger than ${FILE_LIMIT} bytes.`);
          return null;
        }
        return await readFile(file, 'utf8');
      } catch {
        issue(`${relative}: could not read evidence file.`);
        return null;
      }
    }
    async function jsonFile(relative) {
      const text = await contents(relative);
      if (text === null) return null;
      try { return JSON.parse(text); } catch {
        issue(`${relative}: invalid JSON.`);
        return null;
      }
    }
    async function jsonLines(relative) {
      const text = await contents(relative);
      if (text === null) return [];
      const lines = text.split('\n');
      if (lines.at(-1) === '') lines.pop();
      const rows = [];
      for (let index = 0; index < lines.length; index += 1) {
        const where = `${relative}:${index + 1}`;
        try {
          const row = JSON.parse(lines[index]);
          if (!object(row)) issue(`${where}: expected a JSON object.`);
          else rows.push({ row, where });
        } catch { issue(`${where}: invalid JSONL record.`); }
      }
      return rows;
    }

    const manifest = await jsonFile('manifest.json');
    if (!object(manifest) || typeof manifest.runId !== 'string' || !manifest.runId
      || !Array.isArray(manifest.agentIds) || manifest.agentIds.length !== 10
      || manifest.agentIds.some((id) => typeof id !== 'string' || !agentPattern.test(id))
      || new Set(manifest.agentIds).size !== 10 || manifest.pingsPerAgent !== 100
      || manifest.action !== 'switchboard.ping' || manifest.target !== 'gate:test'
      || !Array.isArray(manifest.rounds) || manifest.rounds.length !== 10) {
      issue('manifest.json: invalid run identity, ten-agent roster, ping plan, or ten-round plan.');
      return finish();
    }
    expectedPings = manifest.agentIds.length * manifest.rounds.length * manifest.pingsPerAgent;
    const agentIds = new Set(manifest.agentIds);
    const rounds = new Map();
    let validPlan = true;
    for (const round of manifest.rounds) {
      if (!object(round) || !Number.isInteger(round.round) || round.round < 1 || round.round > 10
        || rounds.has(round.round) || !finite(round.intervalMs)
        || !integer(round.whitelistCount) || round.whitelistCount > 10
        || !object(round.registry) || round.registry.id !== 'apex.stress'
        || round.registry.version !== round.round || !Array.isArray(round.registry.agents)
        || round.registry.agents.length !== 10) {
        issue('manifest.json: invalid or duplicate round/registry definition.');
        validPlan = false;
        continue;
      }
      const principals = new Map();
      for (const agent of round.registry.agents) {
        const expectedGrant = object(agent) ? {
          id: `grant.${agent.id}`, action: manifest.action, target: manifest.target, issuedBy: 'operator:stress',
        } : null;
        if (!object(agent) || !agentIds.has(agent.id) || principals.has(agent.id)
          || typeof agent.whitelisted !== 'boolean' || !Array.isArray(agent.grants)
          || agent.grants.length !== 1 || !isDeepStrictEqual(agent.grants[0], expectedGrant)) {
          issue(`manifest.json: invalid agent or exact grant in round ${round.round}.`);
          validPlan = false;
          continue;
        }
        principals.set(agent.id, agent);
      }
      if (principals.size !== 10
        || [...principals.values()].filter((agent) => agent.whitelisted).length !== round.whitelistCount) {
        issue(`manifest.json: roster or whitelist count disagrees in round ${round.round}.`);
        validPlan = false;
      }
      rounds.set(round.round, { ...round, principals });
    }
    if (!validPlan || rounds.size !== 10) return finish();
    const orderedRounds = [...rounds.values()].sort((a, b) => a.round - b.round);
    for (let index = 1; index < orderedRounds.length; index += 1) {
      if (orderedRounds[index].intervalMs >= orderedRounds[index - 1].intervalMs) {
        issue('manifest.json: requested intervals must strictly decrease each round.');
      }
    }
    for (const round of orderedRounds) {
      const filename = `registry-round-${String(round.round).padStart(2, '0')}.json`;
      if (!isDeepStrictEqual(await jsonFile(filename), round.registry)) {
        issue(`${filename}: registry snapshot disagrees with the manifest.`);
      }
    }

    const roster = await jsonFile('roster.json');
    const pids = new Map();
    const distinctPids = new Set();
    if (!Array.isArray(roster) || roster.length !== 10) {
      issue('roster.json: exactly ten process identities are required.');
      return finish();
    }
    for (const entry of roster) {
      if (!object(entry) || !agentIds.has(entry.agentId) || pids.has(entry.agentId)
        || !Number.isSafeInteger(entry.pid) || entry.pid <= 0 || distinctPids.has(entry.pid)) {
        issue('roster.json: invalid, duplicated, or missing agent/PID association.');
        continue;
      }
      pids.set(entry.agentId, entry.pid);
      distinctPids.add(entry.pid);
    }
    if (pids.size !== 10 || distinctPids.size !== 10) return finish();

    const runStatus = await jsonFile('run-status.json');
    if (!object(runStatus) || runStatus.completed !== true
      || !Array.isArray(runStatus.exits) || runStatus.exits.length !== 10
      || !Array.isArray(runStatus.roundTimings) || runStatus.roundTimings.length !== 10) {
      issue('run-status.json: a completed run, ten worker exits, and ten round timings are required.');
    } else {
      const exited = new Set();
      for (const entry of runStatus.exits) {
        if (!object(entry) || !agentIds.has(entry.agentId) || exited.has(entry.agentId)
          || entry.pid !== pids.get(entry.agentId) || entry.code !== 0 || entry.signal !== null) {
          issue('run-status.json: a worker exit has an invalid identity, PID, code, or signal.');
        } else exited.add(entry.agentId);
      }
      const timed = new Set();
      for (const timing of runStatus.roundTimings) {
        if (!object(timing) || !rounds.has(timing.round) || timed.has(timing.round)
          || !finite(timing.elapsedMs) || timing.elapsedMs <= 0
          || !finite(timing.completedPingsPerSecond) || timing.completedPingsPerSecond <= 0) {
          issue('run-status.json: invalid, duplicated, or nonpositive round timing.');
        } else {
          timed.add(timing.round);
          const expectedRate = 1000 / timing.elapsedMs * manifest.pingsPerAgent * manifest.agentIds.length;
          if (Math.abs(timing.completedPingsPerSecond - expectedRate) > Math.max(0.000001, expectedRate * 0.000001)) {
            issue(`run-status.json: completed ping rate disagrees with round ${timing.round} duration.`);
          }
        }
      }
    }

    try {
      const files = await readdir(join(directory, 'agents'));
      const expectedFiles = new Set(manifest.agentIds.flatMap((id) => [`${id}.jsonl`, `${id}.summary.json`]));
      for (const file of files) if (!expectedFiles.has(file)) issue('agents/: unexpected evidence file or directory.');
    } catch { issue('agents/: could not inspect the agent evidence directory.'); }

    const records = {
      send: new Map(), decision: new Map(), response_sent: new Map(), response: new Map(),
    };
    const complete = new Map();
    const lifecycle = new Map();
    const keyFor = (round, agentId, seq) => `${manifest.runId}/${round}/${agentId}/${seq}`;
    function envelope(row, where, fileAgent = null) {
      if (row.runId !== manifest.runId || !rounds.has(row.round) || !agentIds.has(row.agentId)
        || row.pid !== pids.get(row.agentId) || (fileAgent !== null && row.agentId !== fileAgent)
        || !integer(row.seq) || row.seq >= manifest.pingsPerAgent
        || row.pingId !== keyFor(row.round, row.agentId, row.seq)) {
        issue(`${where}: ping identity, round, sequence, ID, or PID disagrees with the plan.`);
        return false;
      }
      return true;
    }
    function keep(map, key, record) {
      const prior = map.get(key);
      if (prior) prior.count += 1;
      else map.set(key, { count: 1, ...record });
    }
    const expectedIntent = (agentId) => ({ agentId, action: manifest.action, target: manifest.target });
    function validResult(value, row, where) {
      if (!fieldsEqual(value, resultFields)) {
        issue(`${where}: result has an invalid field set.`);
        return false;
      }
      const round = rounds.get(row.round);
      const agent = round.principals.get(row.agentId);
      const expected = {
        stage: 'SWITCHBOARD',
        status: agent.whitelisted ? 'PASS' : 'DENY',
        code: agent.whitelisted ? 'SCOPE_GRANTED' : 'NOT_WHITELISTED',
        registry: { id: round.registry.id, version: round.registry.version },
        principalId: row.agentId,
        intent: agent.whitelisted ? expectedIntent(row.agentId) : null,
        grant: agent.whitelisted ? agent.grants[0] : null,
        next: agent.whitelisted ? 'CONSTRAINT_SCREENING' : null,
      };
      if (typeof value.reason !== 'string' || !value.reason
        || Object.entries(expected).some(([field, wanted]) => !isDeepStrictEqual(value[field], wanted))) {
        issue(`${where}: result disagrees with independently derived whitelist/scope expectations.`);
        return false;
      }
      return true;
    }

    for (const { row, where } of await jsonLines('gate.jsonl')) {
      if (!['decision', 'response_sent', 'send_error'].includes(row.event)) {
        issue(`${where}: unexpected gate event.`);
        continue;
      }
      if (!envelope(row, where)) continue;
      if (row.event === 'send_error') {
        issue(`${where}: gate reported a response-send error.`);
        continue;
      }
      let valid = true;
      if (row.event === 'decision') {
        valid = validResult(row.result, row, where);
        if (!isDeepStrictEqual(row.intent, expectedIntent(row.agentId))) {
          issue(`${where}: gate-received intent disagrees with the planned intent.`);
          valid = false;
        }
        if (!finite(row.evaluateMs)) {
          issue(`${where}: invalid evaluation duration.`);
          valid = false;
        }
      }
      keep(records[row.event], row.pingId, { row, valid });
    }

    for (const agentId of manifest.agentIds) {
      const loggedCompletions = [];
      for (const { row, where } of await jsonLines(`agents/${agentId}.jsonl`)) {
        if (['ready', 'done'].includes(row.event)) {
          if (row.runId !== manifest.runId || row.agentId !== agentId || row.pid !== pids.get(agentId)
            || (row.event === 'done' && row.rounds !== 10)) {
            issue(`${where}: worker lifecycle identity, PID, or completed round count disagrees with the plan.`);
          }
          keep(lifecycle, `${agentId}/${row.event}`, { row });
          continue;
        }
        if (row.event === 'round_complete') {
          if (row.runId !== manifest.runId || !rounds.has(row.round) || row.agentId !== agentId
            || row.pid !== pids.get(agentId)) {
            issue(`${where}: round completion identity or PID disagrees with the plan.`);
            continue;
          }
          if (row.sent !== manifest.pingsPerAgent || row.received !== manifest.pingsPerAgent
            || !integer(row.backpressure) || row.backpressure > manifest.pingsPerAgent
            || !integer(row.maxOutstanding) || row.maxOutstanding > manifest.pingsPerAgent
            || !finite(row.sendSpanMs) || !finite(row.elapsedMs)
            || row.elapsedMs + 0.001 < row.sendSpanMs) {
            issue(`${where}: incomplete or invalid round completion counts/metrics.`);
          }
          keep(complete, `${row.round}/${agentId}`, { row });
          loggedCompletions.push(row);
          continue;
        }
        if (!['send', 'response', 'send_error', 'timeout'].includes(row.event)) {
          issue(`${where}: unexpected agent event.`);
          continue;
        }
        if (!envelope(row, where, agentId)) continue;
        if (['send_error', 'timeout'].includes(row.event)) {
          issue(`${where}: agent reported a send error or response timeout.`);
          continue;
        }
        let valid = true;
        if (row.event === 'send') {
          if (!isDeepStrictEqual(row.intent, expectedIntent(agentId))) {
            issue(`${where}: agent-sent intent disagrees with the planned intent.`);
            valid = false;
          }
          const scheduledMs = row.seq * rounds.get(row.round).intervalMs;
          if (!finite(row.scheduledMs) || row.scheduledMs !== scheduledMs
            || !finite(row.sentMs) || row.sentMs + 0.001 < scheduledMs) {
            issue(`${where}: requested or actual send timing disagrees with the round schedule.`);
            valid = false;
          }
        } else {
          valid = validResult(row.result, row, where);
          if (!finite(row.rttMs)) {
            issue(`${where}: invalid response latency.`);
            valid = false;
          }
        }
        keep(records[row.event], row.pingId, { row, valid });
      }
      for (const event of ['ready', 'done']) {
        const count = lifecycle.get(`${agentId}/${event}`)?.count ?? 0;
        if (count !== 1) issue(`Agent ${agentId}: expected one ${event} record; found ${count}.`);
      }
      const summaryFile = `agents/${agentId}.summary.json`;
      const summary = await jsonFile(summaryFile);
      if (!fieldsEqual(summary, ['runId', 'agentId', 'pid', 'rounds'])
        || summary.runId !== manifest.runId || summary.agentId !== agentId || summary.pid !== pids.get(agentId)
        || !Array.isArray(summary.rounds) || summary.rounds.length !== 10
        || !isDeepStrictEqual(summary.rounds, loggedCompletions)) {
        issue(`${summaryFile}: summary differs from the worker identity or its ten logged round completions.`);
      }
    }

    for (const round of orderedRounds) {
      let pass = 0;
      let deny = 0;
      let matchedPings = 0;
      let backpressure = 0;
      let maxOutstanding = 0;
      const rtts = [];
      const evaluations = [];
      const sendSpans = [];
      for (const agentId of manifest.agentIds) {
        const sendTimes = [];
        for (let seq = 0; seq < manifest.pingsPerAgent; seq += 1) {
          const key = keyFor(round.round, agentId, seq);
          const entries = Object.fromEntries(Object.entries(records).map(([event, map]) => [event, map.get(key)]));
          let matched = true;
          for (const [event, entry] of Object.entries(entries)) {
            if (entry?.count !== 1) {
              issue(`Round ${round.round}, ${agentId}, ping ${seq}: expected one ${event}; found ${entry?.count ?? 0}.`);
              matched = false;
            }
            if (!entry?.valid) matched = false;
          }
          const gate = entries.decision;
          const response = entries.response;
          if (gate?.count === 1) {
            if (gate.row.result?.status === 'PASS') pass += 1;
            if (gate.row.result?.status === 'DENY') deny += 1;
            if (finite(gate.row.evaluateMs)) evaluations.push(gate.row.evaluateMs);
          }
          if (response?.count === 1 && finite(response.row.rttMs)) rtts.push(response.row.rttMs);
          if (entries.send?.count === 1 && finite(entries.send.row.sentMs)) sendTimes.push(entries.send.row.sentMs);
          if (gate && response && !isDeepStrictEqual(gate.row.result, response.row.result)) {
            issue(`Round ${round.round}, ${agentId}, ping ${seq}: agent response differs from gate result.`);
            matched = false;
          }
          if (matched) matchedPings += 1;
        }
        const completion = complete.get(`${round.round}/${agentId}`);
        if (completion?.count !== 1) {
          issue(`Round ${round.round}, ${agentId}: expected one round_complete; found ${completion?.count ?? 0}.`);
        }
        if (completion?.count === 1) {
          if (integer(completion.row.backpressure)) backpressure += completion.row.backpressure;
          if (integer(completion.row.maxOutstanding)) maxOutstanding = Math.max(maxOutstanding, completion.row.maxOutstanding);
        }
        if (sendTimes.length === manifest.pingsPerAgent) {
          const span = Math.max(...sendTimes) - Math.min(...sendTimes);
          sendSpans.push(span);
          if (completion?.count === 1 && finite(completion.row.sendSpanMs)
            && Math.abs(completion.row.sendSpanMs - span) > 1) {
            issue(`Round ${round.round}, ${agentId}: completion send span disagrees with send records by more than 1 ms.`);
          }
        }
      }
      summaries.push({
        round: round.round, intervalMs: round.intervalMs, whitelistCount: round.whitelistCount,
        pass, deny, matchedPings, rttMs: distribution(rtts), evaluationMs: distribution(evaluations),
        sendSpanMs: { min: sendSpans.length ? Math.min(...sendSpans) : null, max: sendSpans.length ? Math.max(...sendSpans) : null },
        backpressure, maxOutstanding,
      });
    }
  } catch {
    issue('Verification could not complete because evidence processing failed.');
  }
  return finish();
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  const report = await verifyRun(process.argv.length === 3 ? process.argv[2] : undefined);
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  process.exitCode = report.ok ? 0 : 1;
}
