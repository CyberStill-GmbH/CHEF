import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import type { Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import { after, before, describe, it } from 'node:test';

import { loadContracts } from '../packages/adapters/src/contracts.js';
import { Sha256Identity } from '../packages/adapters/src/identity.js';
import {
  InMemoryProjectRepository,
  InMemorySnapshotStore,
} from '../packages/adapters/src/memory-projects.js';
import { NmapXmlParser } from '../packages/adapters/src/nmap.js';
import { ExactScopeGuard } from '../packages/adapters/src/scope.js';
import { SessionTokenService } from '../packages/adapters/src/session-token.js';
import { CreateProject } from '../packages/application/src/create-project.js';
import { ImportEvidence } from '../packages/application/src/import-evidence.js';
import { ImportIntoProject } from '../packages/application/src/import-into-project.js';
import { ListProjects } from '../packages/application/src/list-projects.js';
import { ReadProjectSnapshot } from '../packages/application/src/read-project-snapshot.js';
import { ChefError } from '../packages/domain/src/errors.js';
import type {
  ClockPort,
  OAuthProvider,
  RandomPort,
} from '../packages/domain/src/ports.js';
import type { ApiDependencies } from '../apps/api/src/server.js';
import { createChefApi } from '../apps/api/src/server.js';

const contracts = loadContracts('docs/contracts/snapshot.schema.json');
const scopePolicy = contracts.scope(
  JSON.parse(readFileSync('fixtures/scope/lab.json', 'utf8')),
);
const labXml = readFileSync('fixtures/nmap/lab.xml');

const clock: ClockPort = { now: () => '2026-10-30T00:00:00.000Z' };
const identity = new Sha256Identity();
const random: RandomPort = {
  token: () => 'nonce-fixed',
};

const oauth: OAuthProvider = {
  name: 'github',
  authorizeUrl: (state) => `https://github.example/authorize?state=${state}`,
  exchange: async (code) => {
    if (code === 'rejected')
      throw new ChefError('UPSTREAM_ERROR', 'GitHub rejected the code');
    return code === 'second'
      ? { providerUserId: '2', login: 'second-user' }
      : { providerUserId: '1', login: 'first-user' };
  },
};

function dependencies(maxBodyBytes?: number): ApiDependencies {
  const projects = new InMemoryProjectRepository();
  const snapshots = new InMemorySnapshotStore();
  const base: ApiDependencies = {
    createProject: new CreateProject(projects, identity, clock, random),
    listProjects: new ListProjects(projects),
    importIntoProject: new ImportIntoProject(
      projects,
      snapshots,
      new ImportEvidence(
        new NmapXmlParser(),
        new ExactScopeGuard(),
        identity,
        clock,
      ),
    ),
    readSnapshot: new ReadProjectSnapshot(projects, snapshots),
    sessions: new SessionTokenService({
      secret: 't'.repeat(32),
      nowMs: () => Date.UTC(2026, 9, 30),
    }),
    oauth,
    scopePolicy,
    browserCookieSecret: 't'.repeat(32),
  };
  return maxBodyBytes === undefined ? base : { ...base, maxBodyBytes };
}

async function start(
  deps: ApiDependencies,
): Promise<{ server: Server; base: string }> {
  const server = createChefApi(deps);
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  if (address === null || typeof address === 'string')
    throw new Error('Expected a TCP address');
  return { server, base: `http://127.0.0.1:${(address as AddressInfo).port}` };
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== 'object' || value === null)
    throw new Error('Expected a JSON object');
  return value as Record<string, unknown>;
}

function text(value: unknown): string {
  if (typeof value !== 'string') throw new Error('Expected a string');
  return value;
}

describe('CHEF API', () => {
  let server: Server;
  let base = '';

  before(async () => {
    const started = await start(dependencies());
    server = started.server;
    base = started.base;
  });

  after(async () => {
    await new Promise<void>((resolve) => server.close(() => resolve()));
  });

  async function call(path: string, init: RequestInit = {}) {
    const response = await fetch(`${base}${path}`, init);
    const raw = await response.text();
    return {
      status: response.status,
      headers: response.headers,
      body: raw === '' ? undefined : (JSON.parse(raw) as unknown),
    };
  }

  async function signIn(code: string): Promise<string> {
    const start1 = await call('/auth/github/start', { method: 'POST' });
    assert.equal(start1.status, 200);
    const state = text(record(start1.body)['state']);
    const callback = await call(
      `/auth/github/callback?code=${code}&state=${encodeURIComponent(state)}`,
    );
    assert.equal(callback.status, 200);
    return text(record(callback.body)['token']);
  }

  function bearer(token: string): Record<string, string> {
    return {
      authorization: `Bearer ${token}`,
      'content-type': 'application/json',
    };
  }

  it('answers health without authentication', async () => {
    const response = await call('/health');
    assert.equal(response.status, 200);
    assert.equal(record(response.body)['status'], 'ok');
  });

  it('sets hardening headers on every response', async () => {
    const response = await call('/health');
    assert.equal(response.headers.get('x-content-type-options'), 'nosniff');
    assert.equal(response.headers.get('cache-control'), 'no-store');
    assert.equal(response.headers.get('referrer-policy'), 'no-referrer');
  });

  it('rejects a request without a bearer token', async () => {
    const response = await call('/projects');
    assert.equal(response.status, 401);
    assert.equal(record(response.body)['error'], 'UNAUTHORIZED');
  });

  it('rejects a tampered bearer token', async () => {
    const token = await signIn('first');
    const response = await call('/projects', {
      headers: { authorization: `Bearer ${token.slice(0, -1)}x` },
    });
    assert.equal(response.status, 401);
  });

  it('rejects an OAuth state that was not issued by this server', async () => {
    const response = await call(
      '/auth/github/callback?code=first&state=forged',
    );
    assert.equal(response.status, 401);
    assert.equal(record(response.body)['error'], 'UNAUTHORIZED');
  });

  it('does not leak an upstream failure detail', async () => {
    const start1 = await call('/auth/github/start', { method: 'POST' });
    const state = text(record(start1.body)['state']);
    const response = await call(
      `/auth/github/callback?code=rejected&state=${encodeURIComponent(state)}`,
    );
    assert.equal(response.status, 502);
    assert.equal(record(response.body)['error'], 'UPSTREAM_ERROR');
  });

  it('creates a project, imports evidence and reads the snapshot', async () => {
    const token = await signIn('first');
    const created = await call('/projects', {
      method: 'POST',
      headers: bearer(token),
      body: JSON.stringify({ name: 'Cliente ACME' }),
    });
    assert.equal(created.status, 201);
    const project = record(record(created.body)['project']);
    assert.equal(project['name'], 'Cliente ACME');
    const projectId = text(project['id']);

    const imported = await call(
      `/projects/${encodeURIComponent(projectId)}/imports`,
      {
        method: 'POST',
        headers: {
          authorization: `Bearer ${token}`,
          'content-type': 'application/xml',
        },
        body: labXml,
      },
    );
    assert.equal(imported.status, 201);
    const summary = record(imported.body);
    assert.ok(Number(summary['observations']) > 0);
    assert.ok(Number(summary['assets']) > 0);

    const snapshot = await call(
      `/projects/${encodeURIComponent(projectId)}/snapshot`,
      {
        headers: { authorization: `Bearer ${token}` },
      },
    );
    assert.equal(snapshot.status, 200);
    const body = record(snapshot.body);
    assert.equal(body['schemaVersion'], '1.0.0');
    contracts.snapshot(body);
  });

  it('is deterministic: importing the same file twice repeats the run id', async () => {
    const token = await signIn('first');
    const created = await call('/projects', {
      method: 'POST',
      headers: bearer(token),
      body: JSON.stringify({ name: 'Repetible' }),
    });
    const projectId = text(record(record(created.body)['project'])['id']);
    const headers = {
      authorization: `Bearer ${token}`,
      'content-type': 'application/xml',
    };
    const first = await call(
      `/projects/${encodeURIComponent(projectId)}/imports`,
      {
        method: 'POST',
        headers,
        body: labXml,
      },
    );
    const second = await call(
      `/projects/${encodeURIComponent(projectId)}/imports`,
      {
        method: 'POST',
        headers,
        body: labXml,
      },
    );
    assert.equal(record(first.body)['runId'], record(second.body)['runId']);
  });

  it('denies another user the snapshot and the import, without disclosing existence', async () => {
    const owner = await signIn('first');
    const intruder = await signIn('second');
    const created = await call('/projects', {
      method: 'POST',
      headers: bearer(owner),
      body: JSON.stringify({ name: 'Privado' }),
    });
    const projectId = text(record(record(created.body)['project'])['id']);

    const read = await call(
      `/projects/${encodeURIComponent(projectId)}/snapshot`,
      {
        headers: { authorization: `Bearer ${intruder}` },
      },
    );
    assert.equal(read.status, 404);
    assert.equal(record(read.body)['error'], 'NOT_FOUND');

    const write = await call(
      `/projects/${encodeURIComponent(projectId)}/imports`,
      {
        method: 'POST',
        headers: {
          authorization: `Bearer ${intruder}`,
          'content-type': 'application/xml',
        },
        body: labXml,
      },
    );
    assert.equal(write.status, 404);
  });

  it('lists only the caller projects', async () => {
    const owner = await signIn('first');
    await call('/projects', {
      method: 'POST',
      headers: bearer(owner),
      body: JSON.stringify({ name: 'Solo mío' }),
    });
    const intruder = await signIn('second');
    const listed = await call('/projects', {
      headers: { authorization: `Bearer ${intruder}` },
    });
    assert.equal(listed.status, 200);
    assert.deepEqual(record(listed.body)['projects'], []);
  });

  it('rejects an import without an XML content type', async () => {
    const token = await signIn('first');
    const created = await call('/projects', {
      method: 'POST',
      headers: bearer(token),
      body: JSON.stringify({ name: 'Tipo' }),
    });
    const projectId = text(record(record(created.body)['project'])['id']);
    const response = await call(
      `/projects/${encodeURIComponent(projectId)}/imports`,
      {
        method: 'POST',
        headers: {
          authorization: `Bearer ${token}`,
          'content-type': 'text/plain',
        },
        body: labXml,
      },
    );
    assert.equal(response.status, 400);
  });

  it('rejects a hostile XML without publishing a partial run', async () => {
    const token = await signIn('first');
    const created = await call('/projects', {
      method: 'POST',
      headers: bearer(token),
      body: JSON.stringify({ name: 'Hostil' }),
    });
    const projectId = text(record(record(created.body)['project'])['id']);
    const hostile = readFileSync('fixtures/nmap/hostile-xxe.xml');
    const imported = await call(
      `/projects/${encodeURIComponent(projectId)}/imports`,
      {
        method: 'POST',
        headers: {
          authorization: `Bearer ${token}`,
          'content-type': 'application/xml',
        },
        body: hostile,
      },
    );
    assert.notEqual(imported.status, 201);
    const snapshot = await call(
      `/projects/${encodeURIComponent(projectId)}/snapshot`,
      {
        headers: { authorization: `Bearer ${token}` },
      },
    );
    assert.equal(snapshot.status, 404);
  });

  it('returns 404 for an unknown endpoint', async () => {
    const response = await call('/nope');
    assert.equal(response.status, 404);
  });
});

describe('CHEF API body limit', () => {
  it('rejects a body above the configured limit', async () => {
    const started = await start(dependencies(256));
    try {
      const startResponse = await fetch(`${started.base}/auth/github/start`, {
        method: 'POST',
      });
      const state = text(
        record((await startResponse.json()) as unknown)['state'],
      );
      const callback = await fetch(
        `${started.base}/auth/github/callback?code=first&state=${encodeURIComponent(state)}`,
      );
      const token = text(record((await callback.json()) as unknown)['token']);

      const created = await fetch(`${started.base}/projects`, {
        method: 'POST',
        headers: {
          authorization: `Bearer ${token}`,
          'content-type': 'application/json',
        },
        body: JSON.stringify({ name: 'Limite' }),
      });
      const projectId = text(
        record(record((await created.json()) as unknown)['project'])['id'],
      );

      const oversized = await fetch(
        `${started.base}/projects/${encodeURIComponent(projectId)}/imports`,
        {
          method: 'POST',
          headers: {
            authorization: `Bearer ${token}`,
            'content-type': 'application/xml',
          },
          body: `<nmaprun>${'a'.repeat(1024)}</nmaprun>`,
        },
      );
      assert.equal(oversized.status, 413);
    } finally {
      await new Promise<void>((resolve) =>
        started.server.close(() => resolve()),
      );
    }
  });
});

describe('private OAuth instance', () => {
  it('issues sessions only for explicitly enabled provider user IDs', async () => {
    const deps = {
      ...dependencies(),
      allowedOAuthUserIds: new Set(['1']),
    };
    const started = await start(deps);
    try {
      for (const [code, expectedStatus] of [
        ['first', 200],
        ['second', 403],
      ] as const) {
        const auth = await fetch(`${started.base}/auth/github/start`, {
          method: 'POST',
        });
        const state = text(record(await auth.json())['state']);
        const result = await fetch(
          `${started.base}/auth/github/callback?code=${code}&state=${encodeURIComponent(state)}`,
        );
        assert.equal(result.status, expectedStatus);
        const body = record(await result.json());
        if (expectedStatus === 403) {
          assert.equal(body['error'], 'FORBIDDEN');
          assert.equal(body['token'], undefined);
        }
      }
      const oldToken = deps.sessions.issue('github:2');
      const revoked = await fetch(`${started.base}/projects`, {
        headers: { authorization: `Bearer ${oldToken}` },
      });
      assert.equal(revoked.status, 403);
    } finally {
      await new Promise<void>((resolve) =>
        started.server.close(() => resolve()),
      );
    }
  });
});

describe('browser sessions', () => {
  const origin = 'http://127.0.0.1:5173';
  const browserHeaders = { origin, 'x-chef-request': '1' };
  function cookies(response: Response) {
    return response.headers
      .getSetCookie()
      .map((value) => value.split(';')[0])
      .filter(Boolean)
      .join('; ');
  }
  async function signIn(base: string, code = 'first') {
    const started = await fetch(`${base}/auth/github/browser/start`, {
      method: 'POST',
      headers: browserHeaders,
    });
    assert.equal(started.status, 200);
    const authorize = text(record(await started.json())['authorizeUrl']);
    const state = new URL(authorize).searchParams.get('state');
    const callback = await fetch(
      `${base}/auth/github/callback?code=${code}&state=${encodeURIComponent(state ?? '')}`,
      {
        redirect: 'manual',
        headers: { cookie: cookies(started) },
      },
    );
    return { started, callback };
  }

  it('binds unique OAuth state to the browser and returns an HttpOnly session without a token URL', async () => {
    const app = await start({ ...dependencies(), webOrigin: origin });
    try {
      const { started, callback } = await signIn(app.base);
      assert.match(
        started.headers.get('set-cookie') ?? '',
        /HttpOnly; SameSite=Lax/,
      );
      assert.equal(callback.status, 303);
      assert.equal(callback.headers.get('location'), `${origin}/`);
      assert.equal(await callback.text(), '');
      assert.match(
        callback.headers.get('set-cookie') ?? '',
        /chef-session=.*HttpOnly/,
      );
      const encryptedCookie = cookies(callback)
        .split('; ')
        .find((part) => part.startsWith('chef-session='));
      assert.ok(encryptedCookie?.startsWith('chef-session=b1.'));
      const altered = await fetch(`${app.base}/auth/session`, {
        headers: {
          'x-chef-request': '1',
          cookie: encryptedCookie?.replace('b1.', 'b1.A') ?? '',
        },
      });
      assert.equal(altered.status, 401);
      const session = await fetch(`${app.base}/auth/session`, {
        headers: { cookie: cookies(callback), 'x-chef-request': '1' },
      });
      assert.deepEqual(await session.json(), { userId: 'github:1' });
      const second = await fetch(`${app.base}/auth/github/browser/start`, {
        method: 'POST',
        headers: browserHeaders,
      });
      assert.notEqual(cookies(started), cookies(second));
    } finally {
      await new Promise<void>((resolve) => app.server.close(() => resolve()));
    }
  });

  it('rejects missing origin, a foreign origin and browser state without its cookie', async () => {
    const app = await start({ ...dependencies(), webOrigin: origin });
    try {
      for (const headers of [
        { 'x-chef-request': '1' },
        { origin: 'https://foreign.invalid', 'x-chef-request': '1' },
        { origin },
      ]) {
        const denied = await fetch(`${app.base}/auth/github/browser/start`, {
          method: 'POST',
          headers,
        });
        assert.equal(denied.status, 403);
      }
      const started = await fetch(`${app.base}/auth/github/browser/start`, {
        method: 'POST',
        headers: browserHeaders,
      });
      const state =
        new URL(
          text(record(await started.json())['authorizeUrl']),
        ).searchParams.get('state') ?? '';
      const noCookie = await fetch(
        `${app.base}/auth/github/callback?code=first&state=${encodeURIComponent(state)}`,
        { redirect: 'manual' },
      );
      assert.equal(noCookie.status, 303);
      assert.equal(noCookie.headers.get('location'), `${origin}/?auth=failed`);
      const changed = await fetch(
        `${app.base}/auth/github/callback?code=first&state=wrong`,
        { redirect: 'manual', headers: { cookie: cookies(started) } },
      );
      assert.equal(changed.headers.get('location'), `${origin}/?auth=failed`);
      assert.ok(!cookies(changed).includes('chef-session='));
    } finally {
      await new Promise<void>((resolve) => app.server.close(() => resolve()));
    }
  });

  it('protects cookie-authenticated writes and clears the browser session at logout', async () => {
    const app = await start({ ...dependencies(), webOrigin: origin });
    try {
      const { callback } = await signIn(app.base);
      const sessionCookie = cookies(callback);
      const rejected = await fetch(`${app.base}/projects`, {
        method: 'POST',
        headers: {
          cookie: sessionCookie,
          origin: 'https://foreign.invalid',
          'x-chef-request': '1',
          'content-type': 'application/json',
        },
        body: JSON.stringify({ name: 'Blocked' }),
      });
      assert.equal(rejected.status, 403);
      const noHeader = await fetch(`${app.base}/projects`, {
        headers: { cookie: sessionCookie },
      });
      assert.equal(noHeader.status, 403);
      const created = await fetch(`${app.base}/projects`, {
        method: 'POST',
        headers: {
          ...browserHeaders,
          cookie: sessionCookie,
          'content-type': 'application/json',
        },
        body: JSON.stringify({ name: 'Browser project' }),
      });
      assert.equal(created.status, 201);
      const logout = await fetch(`${app.base}/auth/logout`, {
        method: 'POST',
        headers: { ...browserHeaders, cookie: sessionCookie },
      });
      assert.equal(logout.status, 200);
      assert.match(
        logout.headers.get('set-cookie') ?? '',
        /chef-session=;.*Max-Age=0/,
      );
      assert.equal(
        (
          await fetch(`${app.base}/auth/session`, {
            headers: { 'x-chef-request': '1' },
          })
        ).status,
        401,
      );
    } finally {
      await new Promise<void>((resolve) => app.server.close(() => resolve()));
    }
  });

  it('sets Secure host cookies for HTTPS and rejects accounts outside the allowlist', async () => {
    const secureOrigin = 'https://chef.example.invalid';
    const app = await start({
      ...dependencies(),
      webOrigin: secureOrigin,
      allowedOAuthUserIds: new Set(['1']),
    });
    try {
      const started = await fetch(`${app.base}/auth/github/browser/start`, {
        method: 'POST',
        headers: { origin: secureOrigin, 'x-chef-request': '1' },
      });
      assert.match(
        started.headers.get('set-cookie') ?? '',
        /__Host-chef-oauth=.*; Secure/,
      );
      const state =
        new URL(
          text(record(await started.json())['authorizeUrl']),
        ).searchParams.get('state') ?? '';
      const denied = await fetch(
        `${app.base}/auth/github/callback?code=second&state=${encodeURIComponent(state)}`,
        { redirect: 'manual', headers: { cookie: cookies(started) } },
      );
      assert.equal(
        denied.headers.get('location'),
        `${secureOrigin}/?auth=failed`,
      );
      assert.ok(!cookies(denied).includes('__Host-chef-session='));
    } finally {
      await new Promise<void>((resolve) => app.server.close(() => resolve()));
    }
  });
});
