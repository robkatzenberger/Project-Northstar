import { randomBytes, createHash, timingSafeEqual } from 'node:crypto';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, join, resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadRegistry } from './registry-store.js';
import { createTokenAuthenticator } from './switchboard.js';
import { createGate } from './gate.js';
import { createAuditorStore, fault } from './auditor-store.js';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const contentTypes = { '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8', '.js': 'text/javascript; charset=utf-8' };
const digest = (value) => createHash('sha256').update(value).digest();
const sameKey = (left, right) => typeof left === 'string' && left.length === 64 && timingSafeEqual(digest(left), digest(right));

async function body(request) {
  if (!request.headers['content-type']?.startsWith('application/json')) throw fault('JSON_REQUIRED', 'Send application/json.', 415);
  const chunks = [];
  let bytes = 0;
  for await (const chunk of request) {
    bytes += chunk.length;
    if (bytes > 65536) throw fault('REQUEST_TOO_LARGE', 'Request exceeds 64 KiB.', 413);
    chunks.push(chunk);
  }
  try { return JSON.parse(Buffer.concat(chunks).toString('utf8')); }
  catch { throw fault('INVALID_JSON', 'Request must contain valid JSON.'); }
}
function send(response, status, value, headers = {}) {
  response.writeHead(status, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store', ...headers });
  response.end(JSON.stringify(value));
}

export async function createAuditorServer({ dataDir = join(root, 'var/auditor'), registryPath = join(root, 'config/agents.json'), operatorKey } = {}) {
  const store = await createAuditorStore(dataDir, await loadRegistry(registryPath));
  const keyPath = join(dataDir, 'operator.key');
  if (!operatorKey) {
    try { operatorKey = (await readFile(keyPath, 'utf8')).trim(); } catch (error) {
      if (error.code !== 'ENOENT') throw error;
      operatorKey = randomBytes(32).toString('hex');
      await writeFile(keyPath, `${operatorKey}\n`, { flag: 'wx', mode: 0o600 });
    }
  }
  if (!/^[0-9a-f]{64}$/.test(operatorKey)) throw new Error('Operator key must contain 64 lowercase hexadecimal characters.');
  const session = randomBytes(32).toString('hex');
  const credentials = new Map();
  let authenticate = createTokenAuthenticator([]);
  const catalog = JSON.parse(await readFile(join(root, 'config/action-catalog.json'), 'utf8'));
  const allowedStatic = new Set(['index.html', 'app.js', 'styles.css', 'gate.html', 'gate.js', 'gate.css']);
  const server = createServer(async (request, response) => {
    response.setHeader('X-Content-Type-Options', 'nosniff');
    response.setHeader('Referrer-Policy', 'no-referrer');
    response.setHeader('X-Frame-Options', 'DENY');
    try {
      const base = new URL(`http://${request.headers.host}`);
      if (!['127.0.0.1', 'localhost', '[::1]'].includes(base.hostname)) throw fault('INVALID_HOST', 'Use the local auditor address.', 403);
      if (request.headers.origin && request.headers.origin !== base.origin) throw fault('INVALID_ORIGIN', 'Cross-origin requests are not accepted.', 403);
      const url = new URL(request.url, base);
      const cookie = (request.headers.cookie || '').split(';').map((part) => part.trim()).find((part) => part.startsWith('apex_operator='))?.slice(14);
      const bearer = request.headers.authorization?.replace(/^Bearer /, '');
      const isOperator = sameKey(cookie, session) || sameKey(bearer, operatorKey);
      const requireOperator = () => { if (!isOperator) throw fault('OPERATOR_REQUIRED', 'Sign in with the auditor operator key.', 401); };
      if (url.pathname === '/api/operator/session' && request.method === 'POST') {
        const input = await body(request);
        if (!sameKey(input?.key, operatorKey)) throw fault('INVALID_OPERATOR_KEY', 'Operator key was not accepted.', 401);
        send(response, 200, { ok: true }, { 'set-cookie': `apex_operator=${session}; HttpOnly; SameSite=Strict; Path=/; Max-Age=28800` });
        return;
      }
      if (url.pathname === '/api/action-catalog' && request.method === 'GET') {
        send(response, 200, { ...catalog, runtimeEnforcement: 'action_ids_and_exact_scope_rules', versionLabel: '0.0.2' });
        return;
      }
      if (url.pathname === '/api/gate/evaluate' && request.method === 'POST') {
        const input = await body(request);
        const declared = input?.intent;
        const credentialInDeclaration = typeof input?.credential === 'string' && input.credential.length > 0
          && JSON.stringify(declared ?? null).includes(input.credential);
        const safeDeclaration = !credentialInDeclaration && declared && ['agentId', 'action', 'target'].every((key) => typeof declared[key] === 'string' && declared[key].length <= 256)
          ? Object.fromEntries(['agentId', 'action', 'target'].map((key) => [key, declared[key]])) : null;
        // Keep actual credential verification, but never echo/persist a secret
        // accidentally pasted into declaration fields. Null intent blocks.
        const screenedInput = credentialInDeclaration ? { credential: input.credential, intent: null } : input;
        const result = await store.assess(({ version, text }, registry) => createGate({ registry, authenticate, policy: { version, text } }).evaluate(screenedInput), safeDeclaration);
        send(response, 200, result);
        return;
      }
      if (url.pathname.startsWith('/api/agent/assessments/') && request.method === 'GET') {
        const principalId = authenticate(bearer);
        if (!principalId) throw fault('AGENT_REQUIRED', 'A valid agent credential is required.', 401);
        const assessmentId = url.pathname.slice('/api/agent/assessments/'.length);
        send(response, 200, store.getAgentAssessment(assessmentId, principalId));
        return;
      }
      if (url.pathname === '/api/gate-rules') {
        requireOperator();
        if (request.method === 'GET') { send(response, 200, store.getPolicy()); return; }
        if (request.method === 'PUT') {
          const input = await body(request);
          send(response, 200, await store.savePolicy(input?.version, input?.text));
          return;
        }
      }
      if (url.pathname === '/api/audit' && request.method === 'GET') {
        requireOperator();
        const limit = Number(url.searchParams.get('limit') || 20);
        send(response, 200, { records: store.history(Number.isSafeInteger(limit) ? limit : 20) });
        return;
      }
      if (url.pathname === '/api/reviews' && request.method === 'GET') {
        requireOperator();
        send(response, 200, { reviews: store.listReviews() });
        return;
      }
      if (url.pathname === '/api/reviews/decision' && request.method === 'POST') {
        requireOperator();
        // The authenticated operator resolves a saved assessment. The caller
        // cannot replace its intent, policy, verified principal, or reviewer.
        const review = await store.resolveReview(await body(request));
        send(response, 201, { review });
        return;
      }
      if (url.pathname === '/api/gate/credentials' && request.method === 'POST') {
        requireOperator();
        const input = await body(request);
        const token = randomBytes(32).toString('hex');
        await store.issueCredential(input?.agentId, () => {
          credentials.set(input.agentId, token);
          authenticate = createTokenAuthenticator([...credentials].map(([principalId, value]) => ({ principalId, token: value })));
        });
        send(response, 201, { credential: token, principalId: input.agentId, ephemeral: true });
        return;
      }
      if (url.pathname === '/api/registry') {
        requireOperator();
        if (request.method === 'GET') { send(response, 200, { registry: store.getRegistry() }); return; }
        if (request.method === 'PUT') {
          const input = await body(request);
          const result = await store.saveRegistry(input?.registry);
          send(response, 200, result);
          return;
        }
      }
      // The previous operator's token utility is retained. Auditor enrollment
      // uses the explicit principal-bound endpoint above.
      if (url.pathname === '/api/credentials' && request.method === 'POST') {
        requireOperator();
        send(response, 201, { credential: randomBytes(32).toString('hex'), registered: false });
        return;
      }
      if (request.method === 'GET' && url.pathname === '/agent-instructions.md') {
        const contents = await readFile(join(root, 'docs/agent-instructions.md'));
        response.writeHead(200, { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' });
        response.end(contents);
        return;
      }
      const file = url.pathname === '/' ? 'gate.html' : url.pathname.slice(1);
      if (request.method === 'GET' && allowedStatic.has(file)) {
        const contents = await readFile(join(root, 'operator', file));
        response.writeHead(200, { 'content-type': contentTypes[extname(file)], 'cache-control': 'no-store',
          'content-security-policy': "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'" });
        response.end(contents);
        return;
      }
      send(response, 404, { code: 'NOT_FOUND', error: 'Route not found.' });
    } catch (error) {
      if (response.headersSent || response.destroyed) { response.destroy(); return; }
      send(response, error.status || (error.code === 'INVALID_RULES' ? 400 : 503), {
        status: 'ERROR', code: error.code || 'AUDITOR_UNAVAILABLE',
        error: error.status || error.code === 'INVALID_RULES' ? error.message : 'Auditor could not complete the request.',
        ...(error.line ? { line: error.line } : {}), execution: 'NOT_EXECUTED',
      });
    }
  });
  server.requestTimeout = 15000;
  server.headersTimeout = 10000;
  server.on('close', () => { credentials.clear(); authenticate = createTokenAuthenticator([]); });
  return { server, keyPath, store };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const demo = process.argv.includes('--demo');
  const dataDir = resolve(process.env.APEX_DATA_DIR || join(root, demo ? 'var/auditor-demo' : 'var/auditor'));
  let registryPath = process.env.APEX_REGISTRY_FILE ? resolve(process.env.APEX_REGISTRY_FILE) : join(root, 'config/agents.json');
  if (demo && !process.env.APEX_REGISTRY_FILE) {
    await mkdir(dataDir, { recursive: true, mode: 0o700 });
    registryPath = join(dataDir, 'agents.json');
    try { await writeFile(registryPath, await readFile(join(root, 'examples/auditor-agents.json')), { flag: 'wx', mode: 0o600 }); }
    catch (error) { if (error.code !== 'EEXIST') throw error; }
  }
  const { server, keyPath } = await createAuditorServer({ dataDir, registryPath });
  const port = Number(process.env.PORT || 43129);
  server.listen(port, '127.0.0.1', () => {
    console.log(`Apex Auditor v0.0.2: http://127.0.0.1:${server.address().port}`);
    console.log(`Operator key file: ${keyPath}`);
    console.log('Rules and audit records persist; agent credentials expire when this server stops.');
  });
}
