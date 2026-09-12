// Optional verification dependency; no browser package is used at runtime.
import assert from 'node:assert/strict';
import { createHash, randomBytes } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createAuditorServer } from '../../src/server.js';

const { chromium } = await import(process.env.APEX_PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('../../', import.meta.url));
const output = process.env.APEX_BROWSER_REPORT || join(root, 'var/browser-check', `v0.0.2-${Date.now()}`);
const scratch = await mkdtemp(join(tmpdir(), 'apex-browser-'));
const operatorKey = randomBytes(32).toString('hex');
const { server } = await createAuditorServer({ dataDir: scratch,
  registryPath: join(root, 'examples/auditor-agents.json'), operatorKey });
let browser;
const steps = [], errors = [], credentials = [], observedAssessments = [], observedReviews = [];
const rules = 'APPROVE read_file file:demo\nESCALATE delete_file file:demo\nESCALATE send_email recipient:demo';
const paddedContext = '  Checked the exact demo file scope. <b>context only</b>  ';
try {
  await mkdir(output, { recursive: true });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  const url = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({ headless: true,
    ...(process.env.APEX_BROWSER_EXECUTABLE ? { executablePath: process.env.APEX_BROWSER_EXECUTABLE } : {}) });
  const page = await browser.newPage({ viewport: { width: 1280, height: 1000 } });
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('dialog', (dialog) => dialog.accept());
  const responseFor = (path) => page.waitForResponse((response) => response.url() === `${url}${path}` && response.request().method() === 'POST');
  const cardFor = (id) => page.locator(`.review-card[data-assessment-id="${id}"]`);
  await page.goto(url);
  assert.equal(await page.title(), 'Apex Auditor · v0.0.2');
  assert.equal(await page.locator('#developer-tools').getAttribute('open'), null);
  assert.equal(await page.locator('#evaluate-form').isVisible(), false);
  assert.match(await page.locator('.purpose').textContent(), /Agent requests arrive automatically/);
  await page.locator('#sign-in-panel').waitFor({ state: 'visible' });
  assert.equal(await page.locator('#rules').isDisabled(), true);
  assert.equal(await page.locator('#review-records button').count(), 0);
  assert.equal(await page.locator('#agent-connection').isVisible(), false);
  steps.push('Unauthenticated visitors cannot edit rules or make human decisions.');
  await page.locator('#operator-key').fill(operatorKey);
  await page.locator('#sign-in').click();
  await page.locator('#sign-in-panel').waitFor({ state: 'hidden' });
  await page.waitForFunction(() => !document.querySelector('#rules').disabled);
  assert.equal(await page.locator('#operator-key').inputValue(), '');
  assert.equal(await page.locator('#connection-agent-id').inputValue(), '');
  steps.push('Operator sign-in unlocks rules and clears the entered key.');
  steps.push('Human dashboard is primary; the synthetic developer console starts collapsed and the agent connection has no assumed agent ID.');
  await page.locator('#rules').fill(rules);
  await page.locator('#save-rules').click();
  await page.waitForFunction(() => document.querySelector('#policy-version').textContent === 'Policy version 2');
  await page.locator('#rules').fill('APPROVE read_file *');
  await page.locator('#save-rules').click();
  await page.locator('#rule-error').waitFor({ state: 'visible' });
  assert.match(await page.locator('#rule-error').textContent(), /Line 1/);
  assert.equal(await page.locator('#policy-version').textContent(), 'Policy version 2');
  await page.locator('#reload-rules').click();
  await page.waitForFunction((text) => document.querySelector('#rules').value === text, rules);
  steps.push('APPROVE/ESCALATE rules activate; an invalid wildcard reports line 1 and preserves policy version 2.');

  await page.locator('#connection-agent-id').fill('demo-agent');
  const issuing = responseFor('/api/gate/credentials');
  await page.locator('#issue-agent-credential').click();
  const agentCredential = (await (await issuing).json()).credential;
  credentials.push(agentCredential);
  await page.waitForFunction(() => document.querySelector('#connection-credential').value.length === 64);
  assert.equal(await page.locator('#connection-credential').getAttribute('type'), 'password');
  assert.equal(await page.locator('#connection-credential').getAttribute('readonly'), '');
  assert.equal(await page.locator('#agent-endpoint').textContent(), `${url}/api/gate/evaluate`);
  assert.equal(await page.locator('.instructions-link').getAttribute('href'), '/agent-instructions.md');
  await page.locator('#clear-agent-credential').click();
  assert.equal(await page.locator('#connection-credential').inputValue(), '');
  assert.equal(await page.locator('#connection-secret').isVisible(), false);
  steps.push('Operator provisions a principal-bound credential separately from requests; it is masked, can be cleared, and has an agent endpoint/instructions.');

  async function assess(action, target, expected) {
    // Independent agent HTTP client: no browser cookies or operator key.
    const response = await fetch(`${url}/api/gate/evaluate`, { method: 'POST',
      headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ credential: agentCredential,
        intent: { agentId: 'demo-agent', action, target } }) });
    assert.equal(response.status, 200);
    const result = await response.json();
    assert.equal(result.status, expected);
    assert.equal(result.version, '0.0.2');
    assert.equal(result.decisionModel, 'approve_escalate_v1');
    assert.equal(result.execution, 'NOT_EXECUTED');
    observedAssessments.push({ result, request: { agentId: 'demo-agent', action, target } });
    steps.push(`Agent HTTP request ${action} ${target}: ${expected}, recorded receipt.`);
    return result;
  }
  const automatic = await assess('read_file', 'file:demo', 'APPROVE');
  const approval = await assess('delete_file', 'file:demo', 'ESCALATE');
  await page.waitForFunction((id) => document.querySelector(`[data-assessment-id="${id}"]`)?.dataset.reviewState === 'pending', approval.audit.id);
  assert.equal(await page.locator('#developer-tools').getAttribute('open'), null);
  steps.push('An independent agent request appears in human review automatically without a manual request or Refresh click.');
  const workingContext = 'Reviewing this exact request; keep this unfinished context.';
  await cardFor(approval.audit.id).locator('textarea').fill(workingContext);
  const denial = await assess('send_email', 'recipient:demo', 'ESCALATE');
  await page.locator('#reviews-paused').waitFor({ state: 'visible' });
  assert.equal(await cardFor(approval.audit.id).locator('textarea').inputValue(), workingContext);
  assert.equal(await cardFor(approval.audit.id).locator('textarea').evaluate((element) => document.activeElement === element), true);
  await page.waitForFunction((id) => document.querySelector('#audit-records').textContent.includes(id), denial.audit.id);
  await page.locator('#refresh-reviews').click();
  await cardFor(denial.audit.id).waitFor({ state: 'visible' });
  assert.equal(await cardFor(approval.audit.id).locator('textarea').inputValue(), workingContext);
  await cardFor(approval.audit.id).locator('textarea').fill('');
  await page.locator('#reviews-heading').click();
  steps.push('While human context is being edited, automatic review redraw pauses, focus/text survive, audit still updates, and explicit Refresh preserves the draft.');
  const stale = await assess('write_file', 'file:demo', 'ESCALATE');
  assert.equal(stale.code, 'NO_MATCHING_RULE');
  const blocked = await assess('read_file', 'file:other', 'BLOCKED');
  assert.equal(blocked.switchboard.status, 'DENY');
  const malformed = await assess('read_file', '*', 'ERROR');
  await page.waitForFunction(() => document.querySelectorAll('.review-card[data-review-state="pending"]').length === 3);
  for (const result of [automatic, blocked, malformed]) assert.equal(await cardFor(result.audit.id).count(), 0);
  for (const result of [approval, denial, stale]) {
    const text = await cardFor(result.audit.id).textContent();
    for (const expected of [result.audit.id, result.audit.recordedAt, 'demo-agent', result.switchboard.intent.action,
      result.switchboard.intent.target, 'Verified agent', 'Policy version', result.policy.hash,
      result.switchboard.registry.id, `version ${result.switchboard.registry.version}`, result.reason, 'Matched rules']) assert.ok(text.includes(expected));
    for (const rule of result.matchedRules) assert.ok(text.includes(`Line ${rule.line} · ${rule.decision} ${rule.action} ${rule.target}`));
  }
  steps.push('Only gate escalations enter review; each shows the exact declaration, verified principal, registry, policy, matched rules, reason, and receipt.');
  const approvalCard = cardFor(approval.audit.id);
  await page.waitForLoadState('networkidle');
  await approvalCard.screenshot({ path: join(output, 'human-review.png') });
  await approvalCard.locator('textarea').fill('   ');
  await approvalCard.locator('button[value="APPROVE"]').click();
  assert.equal(await approvalCard.locator('textarea').evaluate((element) => element.checkValidity()), false);
  assert.equal(await approvalCard.getAttribute('data-review-state'), 'pending');
  steps.push('Whitespace-only human context is rejected by the form.');

  async function decide(result, decision, comment) {
    const card = cardFor(result.audit.id);
    await card.locator('textarea').fill(comment);
    const pending = responseFor('/api/reviews/decision');
    await card.locator(`button[value="${decision}"]`).click();
    const response = await pending;
    assert.equal(response.status(), 201);
    const { review } = await response.json();
    assert.equal(review.assessmentId, result.audit.id);
    assert.equal(review.decision, decision);
    assert.equal(review.comment, comment.trim());
    assert.equal(review.reviewer, 'operator');
    assert.equal(review.execution, 'NOT_EXECUTED');
    await page.waitForFunction((id) => document.querySelector(`[data-assessment-id="${id}"]`)?.dataset.reviewState === 'resolved', result.audit.id);
    assert.equal(await card.locator('button').count(), 0);
    const outcome = await card.locator('.review-outcome').textContent();
    assert.ok(outcome.includes(review.id));
    assert.ok(outcome.includes(comment.trim()));
    assert.match(await card.locator('.review-heading').textContent(), /GATE ESCALATE/);
    observedReviews.push(review);
    return review;
  }
  await decide(approval, 'APPROVE', paddedContext);
  assert.equal(await approvalCard.locator('.review-outcome b').count(), 0);
  assert.ok((await approvalCard.locator('.review-outcome .review-comment').textContent()).includes('<b>context only</b>'));
  steps.push('Human APPROVE accepts padded context, stores trimmed text, renders HTML-looking context literally, and preserves the original gate ESCALATE.');
  await decide(denial, 'DENY', 'Recipient verification is incomplete; do not proceed with this intent.');
  steps.push('Human DENY records separate context and outcome bound to the escalation receipt.');

  const updatedRules = `${rules}\n# Reviewed policy revision`;
  await page.locator('#rules').fill(updatedRules);
  await page.locator('#save-rules').click();
  await page.waitForFunction(() => document.querySelector('#policy-version').textContent === 'Policy version 3');
  const staleCard = cardFor(stale.audit.id);
  const staleContext = 'Ready to review the recorded write request.';
  await staleCard.locator('textarea').fill(staleContext);
  const pending = responseFor('/api/reviews/decision');
  await staleCard.locator('button[value="APPROVE"]').click();
  const failed = await pending;
  assert.equal(failed.status(), 409);
  assert.equal((await failed.json()).code, 'REVIEW_STALE');
  await page.waitForFunction((id) => document.querySelector(`[data-assessment-id="${id}"]`)?.dataset.reviewState === 'stale', stale.audit.id);
  assert.equal(await staleCard.locator('textarea').inputValue(), staleContext);
  assert.equal(await staleCard.locator('button[value="APPROVE"]').isDisabled(), true);
  assert.equal(await staleCard.locator('button[value="DENY"]').isEnabled(), true);
  assert.match(await staleCard.locator('.review-error').textContent(), /unsent context is preserved/);
  await decide(stale, 'DENY', 'Policy changed; close this outdated write assessment without approval.');
  steps.push('Stale approval receives 409, refreshes the queue, preserves context, disables Approve, and permits a human Deny.');
  await page.locator('#refresh-audit').click();
  await page.waitForFunction(() => Array.from(document.querySelectorAll('#audit-records strong')).filter((node) => node.textContent === 'Human decision').length === 3);
  assert.equal(await page.locator('#audit-records b').count(), 0);
  await page.screenshot({ path: join(output, 'desktop.png'), fullPage: true });
  await page.reload();
  await page.waitForFunction((text) => document.querySelector('#rules').value === text, updatedRules);
  await page.waitForFunction(() => document.querySelectorAll('.review-card[data-review-state="resolved"]').length === 3);
  assert.equal(await page.locator('#policy-version').textContent(), 'Policy version 3');
  for (const review of observedReviews) {
    const card = cardFor(review.assessmentId);
    const text = await card.locator('.review-outcome').textContent();
    for (const value of [review.id, review.comment, review.decision]) assert.ok(text.includes(value));
    assert.equal(await card.locator('button').count(), 0);
  }
  steps.push('Reload preserves saved rules, session, human outcomes/comments, and receipt binding; resolved reviews remain read-only.');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForLoadState('networkidle');
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth), false);
  await cardFor(approval.audit.id).screenshot({ path: join(output, 'human-outcome-mobile.png') });
  await page.screenshot({ path: join(output, 'mobile.png'), fullPage: true });
  steps.push('390 px layout has no horizontal overflow.');
  assert.deepEqual(errors, []);
  const journal = await readFile(join(scratch, 'audit.jsonl'), 'utf8');
  for (const secret of [operatorKey, ...credentials]) assert.equal(journal.includes(secret), false);
  const records = journal.trim().split('\n').map(JSON.parse);
  const assessments = records.filter((record) => record.type === 'assessment');
  const reviews = records.filter((record) => record.type === 'human_review');
  assert.equal(assessments.length, 6);
  assert.equal(reviews.length, 3);
  assert.deepEqual(assessments.map((record) => record.result.status), ['APPROVE', 'ESCALATE', 'ESCALATE', 'ESCALATE', 'BLOCKED', 'ERROR']);
  for (const observed of observedAssessments) {
    const saved = assessments.find((record) => record.id === observed.result.audit.id);
    const { audit, ...gateResult } = observed.result;
    assert.equal(saved.recordedAt, audit.recordedAt);
    assert.deepEqual(saved.result, gateResult);
    assert.deepEqual(saved.request, observed.request);
  }
  for (const observed of observedReviews) {
    const saved = reviews.find((record) => record.id === observed.id);
    assert.deepEqual(saved, observed);
    const assessment = assessments.find((record) => record.id === saved.assessmentId);
    assert.equal(saved.assessmentHash, createHash('sha256').update(JSON.stringify({ request: assessment.request, result: assessment.result })).digest('hex'));
    assert.equal(assessment.result.status, 'ESCALATE');
  }
  steps.push('Six assessments and three human outcomes reconcile with the journal and exact unchanged assessment hashes; credentials are absent.');
  const report = { recordedAt: new Date().toISOString(), version: '0.0.2', decisionModel: 'approve_escalate_v1',
    status: 'PASS', browser: await browser.version(), steps, assessments: assessments.length,
    humanReviews: reviews.length, pageErrors: errors };
  await writeFile(join(output, 'browser.json'), `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
} finally {
  await browser?.close();
  server.closeAllConnections();
  await new Promise((resolve) => server.close(resolve));
  await rm(scratch, { recursive: true, force: true });
}
