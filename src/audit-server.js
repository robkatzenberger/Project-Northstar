import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  createSamplePending,
  decide,
  defaultListView,
  filterRecords,
  hashIntentSummary,
  loadAuditState,
  seedPendingIfMissing,
} from './audit-store.js';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const auditRoot = join(root, 'audit');
const dataDir = process.env.APEX_AUDIT_DATA || join(auditRoot, 'data');
const port = Number(process.env.PORT || 43128);
const host = '127.0.0.1';
const contentTypes = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.svg': 'image/svg+xml',
};

function sendJson(response, status, body) {
  response.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
  });
  response.end(JSON.stringify(body));
}

async function readBody(request) {
  const chunks = [];
  let size = 0;
  for await (const chunk of request) {
    size += chunk.length;
    if (size > 1_000_000) throw new Error('Request body is too large.');
    chunks.push(chunk);
  }
  if (chunks.length === 0) return {};
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}

async function serveStatic(pathname, response) {
  const relativePath = pathname === '/' ? 'index.html' : pathname.slice(1);
  const filePath = normalize(join(auditRoot, relativePath));
  if (!filePath.startsWith(`${auditRoot}/`)) {
    response.writeHead(404).end();
    return;
  }
  try {
    const info = await stat(filePath);
    if (!info.isFile()) throw new Error('Not a file');
    response.writeHead(200, {
      'content-type': contentTypes[extname(filePath)] || 'application/octet-stream',
    });
    createReadStream(filePath).pipe(response);
  } catch {
    response.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
    response.end('Not found');
  }
}

function listPayload(state, query) {
  const filters = {
    status: query.get('status') || undefined,
    agentId: query.get('agentId') || undefined,
    action: query.get('action') || undefined,
    requestId: query.get('requestId') || undefined,
    since: query.get('since') || undefined,
    hideDenied: false,
  };
  const hasExplicitFilter = Boolean(
    filters.status || filters.agentId || filters.action || filters.requestId || filters.since,
  );
  const pending = hasExplicitFilter
    ? filterRecords(state.pending, filters)
    : defaultListView(state.pending);
  return {
    pending,
    decisions: state.decisions,
    counts: {
      pending: state.pending.filter((item) => item.status === 'PENDING').length,
      approved: state.pending.filter((item) => item.status === 'APPROVED').length,
      denied: state.pending.filter((item) => item.status === 'DENIED').length,
      error: state.pending.filter((item) => item.status === 'ERROR').length,
      total: state.pending.length,
    },
  };
}

const server = createServer(async (request, response) => {
  const url = new URL(request.url, `http://${host}:${port}`);
  try {
    if (request.method === 'GET' && url.pathname === '/api/audit') {
      const state = await loadAuditState(dataDir);
      sendJson(response, 200, listPayload(state, url.searchParams));
      return;
    }

    if (request.method === 'GET' && url.pathname.startsWith('/api/audit/')) {
      const requestId = decodeURIComponent(url.pathname.slice('/api/audit/'.length));
      const state = await loadAuditState(dataDir);
      const record = state.pending.find((item) => item.requestId === requestId);
      if (!record) {
        sendJson(response, 404, { error: `Unknown requestId "${requestId}".` });
        return;
      }
      const decision = state.decisions.find((item) => item.requestId === requestId) || null;
      sendJson(response, 200, {
        record,
        decision,
        intentSummaryHash: hashIntentSummary(record.intentSummary),
      });
      return;
    }

    if (request.method === 'POST' && url.pathname === '/api/audit/decide') {
      const body = await readBody(request);
      const result = await decide(
        dataDir,
        body.requestId,
        body.decision,
        body.operatorId,
        body.intentSummaryHash,
      );
      sendJson(response, 200, {
        receipt: result.receipt,
        pending: defaultListView(result.pending),
        message: 'Decision receipt recorded. This does not mean the action ran.',
      });
      return;
    }

    if (request.method === 'GET') {
      await serveStatic(url.pathname, response);
      return;
    }

    response.writeHead(405, { allow: 'GET, POST' }).end();
  } catch (error) {
    sendJson(response, 400, { error: error.message || 'Request failed.' });
  }
});

await seedPendingIfMissing(dataDir, createSamplePending());

server.listen(port, host, () => {
  console.log(`Apex audit is available at http://${host}:${port}`);
  console.log(`Audit data directory: ${dataDir}`);
});
