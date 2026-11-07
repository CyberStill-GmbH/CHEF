import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { after, test } from 'node:test';

import { loadContracts } from '../packages/adapters/src/contracts.js';
import {
  GoCorrelationEngine,
  type GoCorrelationOptions,
} from '../packages/adapters/src/go-correlation.js';
import { Sha256Identity } from '../packages/adapters/src/identity.js';
import {
  InMemoryProjectRepository,
  InMemorySnapshotStore,
} from '../packages/adapters/src/memory-projects.js';
import { NmapXmlParser } from '../packages/adapters/src/nmap.js';
import { ExactScopeGuard } from '../packages/adapters/src/scope.js';
import { SessionTokenService } from '../packages/adapters/src/session-token.js';
import { createChefApi } from '../apps/api/src/server.js';
import { CreateProject } from '../packages/application/src/create-project.js';
import { ImportEvidence } from '../packages/application/src/import-evidence.js';
import { ImportIntoProject } from '../packages/application/src/import-into-project.js';
import { ListProjects } from '../packages/application/src/list-projects.js';
import { ReadProjectSnapshot } from '../packages/application/src/read-project-snapshot.js';
import { ChefError } from '../packages/domain/src/errors.js';
import { correlate } from '../packages/domain/src/correlate.js';

const identity = new Sha256Identity();
const clock = { now: () => '2026-10-30T00:00:00.000Z' };
const contracts = loadContracts('docs/contracts/snapshot.schema.json');
const policy = contracts.scope(
  JSON.parse(readFileSync('fixtures/scope/lab.json', 'utf8')),
);
const xml = readFileSync('fixtures/nmap/lab.xml');
const importer = new ImportEvidence(
  new NmapXmlParser(),
  new ExactScopeGuard(),
  identity,
  clock,
);
const prepared = importer.prepare(xml, policy);
const expected = importer.execute(xml, policy);
const wireResult = {
  schemaVersion: 'correlation-result/v0',
  ruleSetVersion: 'v1',
  asOf: prepared.run.finishedAt,
  assets: expected.assets,
  relationships: expected.relationships.map(({ assertion, ...r }) => ({
    ...r,
    status: assertion,
  })),
  paths: expected.paths.map(({ assertion, ...p }) => ({
    ...p,
    status: assertion,
    ruleVersion: 'v1',
  })),
  metrics: {
    observationsIn: prepared.observations.length,
    assetsOut: expected.assets.length,
    relationshipsOut: expected.relationships.length,
    pathsOut: expected.paths.length,
    duplicateObservations:
      prepared.observations.length -
      expected.assets.filter((a) => a.kind === 'service').length,
  },
};
const fixtureDirectory = mkdtempSync(join(tmpdir(), 'chef-go-'));
// mkdtemp owns this exact directory; never remove an input/configuration path.
after(() => rmSync(fixtureDirectory, { recursive: true, force: true }));
const fakeBinary = join(fixtureDirectory, 'engine with spaces & symbols.mjs');
writeFileSync(
  fakeBinary,
  `
const mode = process.argv[2];
const result = ${JSON.stringify(wireResult)};
let payload = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => payload += chunk);
process.stdin.on('end', () => {
  if (process.env.CHEF_SESSION_SECRET || process.env.GITHUB_CLIENT_SECRET || process.env.DATABASE_URL) process.exit(9);
  if (mode === 'hang') { setInterval(() => {}, 1000); return; }
  if (mode === 'crash') { process.stdout.write(JSON.stringify(result)); process.stderr.write('secret-payload'); process.exitCode = 7; return; }
  if (mode === 'garbage') { process.stdout.write('secret-payload'); return; }
  if (mode === 'large') { process.stdout.write('x'.repeat(100000)); return; }
  if (mode === 'stderr') { process.stderr.write('x'.repeat(100000)); return; }
  if (mode === 'references') result.assets[0].observationIds = ['observation:' + 'f'.repeat(64)];
  if (mode === 'confirmed') result.paths[0].isConfirmedVulnerability = true;
  if (mode === 'version') result.ruleSetVersion = 'v2';
  if (mode === 'missing') delete result.assets;
  if (mode === 'metrics') result.metrics.observationsIn++;
  process.stdout.write(JSON.stringify(result));
});
`,
);

function fake(mode: string, options: Partial<GoCorrelationOptions> = {}) {
  return new GoCorrelationEngine({
    executable: process.execPath,
    args: [fakeBinary, mode],
    ...options,
  });
}

function hasCode(code: string) {
  return (error: unknown) =>
    error instanceof ChefError &&
    error.code === code &&
    !error.message.includes('secret-payload');
}

test('Go binary produces the same snapshot graph as TypeScript', async () => {
  const engine = new GoCorrelationEngine({
    executable: resolve(
      `artifacts/bin/correlate${process.platform === 'win32' ? '.exe' : ''}`,
    ),
  });
  const graph = await engine.correlate(prepared);
  assert.deepEqual({ ...prepared, ...graph }, expected);
  contracts.snapshot({ ...prepared, ...graph });
  const reversed = {
    ...prepared,
    observations: [...prepared.observations].reverse(),
  };
  assert.deepEqual(await engine.correlate(reversed), graph);
});

test('trusted executable path with spaces works without a shell', async () => {
  assert.deepEqual(
    { ...prepared, ...(await fake('ok').correlate(prepared)) },
    expected,
  );
});

test('Go/TS conformance covers IPv6, UDP, duplicates and empty jobs', async () => {
  const engine = new GoCorrelationEngine({
    executable: resolve(
      `artifacts/bin/correlate${process.platform === 'win32' ? '.exe' : ''}`,
    ),
  });
  const base = prepared.observations[0];
  assert.ok(base);
  const observations = policy.allowedAddresses.flatMap((address) =>
    policy.allowedProtocols.flatMap((protocol) =>
      policy.allowedPorts.flatMap((port) =>
        [0, 1].map((duplicate) => {
          const locator = `${address}/${protocol}/${port}/${duplicate}`;
          return {
            ...base,
            address,
            protocol,
            port,
            locator,
            observedAt:
              duplicate === 0
                ? '2026-10-29T00:00:00.000Z'
                : '2026-10-30T00:00:00.000Z',
            id: identity.id('observation', [base.evidenceId, locator]),
          };
        }),
      ),
    ),
  );
  for (const input of [
    { ...prepared, observations },
    { ...prepared, observations: [] },
  ]) {
    const graph = await engine.correlate(input);
    assert.deepEqual(graph, correlate(input.observations, identity));
    contracts.snapshot({ ...input, ...graph });
  }
});

for (const mode of [
  'crash',
  'garbage',
  'references',
  'confirmed',
  'version',
  'missing',
  'metrics',
  'large',
  'stderr',
])
  test(`rejects ${mode} without publishing a snapshot`, async () => {
    const projects = new InMemoryProjectRepository();
    const snapshots = new InMemorySnapshotStore();
    const project = await new CreateProject(projects, identity, clock, {
      token: () => 'nonce',
    }).execute({ name: 'Go test', ownerId: 'owner' });
    // An existing valid snapshot must survive a failed replacement unchanged.
    await snapshots.save(project.id, expected);
    const uc = new ImportIntoProject(
      projects,
      snapshots,
      importer,
      fake(mode, { maxOutputBytes: 16 * 1024 }),
    );
    await assert.rejects(
      uc.execute({
        projectId: project.id,
        userId: 'owner',
        bytes: xml,
        policy,
      }),
      hasCode('UPSTREAM_ERROR'),
    );
    assert.deepEqual(await snapshots.latest(project.id), expected);
  });

test('missing binary is a redacted upstream error', async () => {
  const engine = new GoCorrelationEngine({
    executable: join(fixtureDirectory, 'missing-executable'),
  });
  await assert.rejects(engine.correlate(prepared), hasCode('UPSTREAM_ERROR'));
});

test('timeout kills a stalled child and frees the slot', async () => {
  const engine = fake('hang', { timeoutMs: 250, maxConcurrent: 1 });
  for (let i = 0; i < 2; i++)
    await assert.rejects(engine.correlate(prepared), hasCode('UPSTREAM_ERROR'));
});

test('cancellation and concurrency are bounded, with no unbounded queue', async () => {
  const engine = fake('hang', { timeoutMs: 5000, maxConcurrent: 1 });
  const cancellation = new AbortController();
  const pending = engine.correlate(prepared, cancellation.signal);
  await assert.rejects(
    engine.correlate(prepared),
    hasCode('UPSTREAM_UNAVAILABLE'),
  );
  cancellation.abort();
  await assert.rejects(pending, hasCode('CANCELLED'));
  await assert.rejects(
    engine.correlate(prepared, cancellation.signal),
    hasCode('CANCELLED'),
  );
});

test('input size and broken evidence references fail before spawning', async () => {
  await assert.rejects(
    fake('ok', { maxInputBytes: 1 }).correlate(prepared),
    hasCode('LIMIT_EXCEEDED'),
  );
  const broken = structuredClone(prepared);
  broken.evidence = [];
  await assert.rejects(fake('ok').correlate(broken), hasCode('INVALID_INPUT'));
});

test('another project member cannot send evidence to the engine', async () => {
  const projects = new InMemoryProjectRepository();
  const snapshots = new InMemorySnapshotStore();
  let nonce = 0;
  const create = new CreateProject(projects, identity, clock, {
    token: () => String(++nonce),
  });
  const a = await create.execute({ name: 'Project A', ownerId: 'alice' });
  await create.execute({ name: 'Project B', ownerId: 'bob' });
  let called = false;
  const uc = new ImportIntoProject(projects, snapshots, importer, {
    correlate: () => {
      called = true;
      throw new Error('must not run');
    },
  });
  await assert.rejects(
    uc.execute({ projectId: a.id, userId: 'bob', bytes: xml, policy }),
    hasCode('NOT_FOUND'),
  );
  assert.equal(called, false);
  assert.equal(await snapshots.latest(a.id), undefined);
});

test('cancellation after engine completion still prevents saving', async () => {
  const projects = new InMemoryProjectRepository();
  const snapshots = new InMemorySnapshotStore();
  const project = await new CreateProject(projects, identity, clock, {
    token: () => 'cancel',
  }).execute({ name: 'Cancel', ownerId: 'owner' });
  const cancellation = new AbortController();
  const uc = new ImportIntoProject(projects, snapshots, importer, {
    correlate: async () => {
      cancellation.abort();
      return expected;
    },
  });
  await assert.rejects(
    uc.execute({
      projectId: project.id,
      userId: 'owner',
      bytes: xml,
      policy,
      signal: cancellation.signal,
    }),
    hasCode('CANCELLED'),
  );
  assert.equal(await snapshots.latest(project.id), undefined);
});

test('HTTP API imports through real Go and denies another user before engine execution', async () => {
  const projects = new InMemoryProjectRepository();
  const snapshots = new InMemorySnapshotStore();
  const createProject = new CreateProject(projects, identity, clock, {
    token: () => 'http',
  });
  const project = await createProject.execute({
    name: 'HTTP Go',
    ownerId: 'owner',
  });
  const sessions = new SessionTokenService({
    secret: 'x'.repeat(32),
    nowMs: () => Date.UTC(2026, 9, 30),
  });
  const engine = new GoCorrelationEngine({
    executable: resolve(
      `artifacts/bin/correlate${process.platform === 'win32' ? '.exe' : ''}`,
    ),
  });
  let calls = 0;
  const server = createChefApi({
    createProject,
    listProjects: new ListProjects(projects),
    importIntoProject: new ImportIntoProject(projects, snapshots, importer, {
      correlate: (input, signal) => {
        calls++;
        return engine.correlate(input, signal);
      },
    }),
    readSnapshot: new ReadProjectSnapshot(projects, snapshots),
    sessions,
    scopePolicy: policy,
    oauth: {
      name: 'fixture',
      authorizeUrl: () => '',
      exchange: async () => {
        throw new Error('unused');
      },
    },
  });
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  assert.ok(address && typeof address !== 'string');
  const base = `http://127.0.0.1:${address.port}/projects/${encodeURIComponent(project.id)}`;
  try {
    const denied = await fetch(`${base}/imports`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${sessions.issue('outsider')}`,
        'content-type': 'application/xml',
      },
      body: xml,
    });
    assert.equal(denied.status, 404);
    assert.equal(calls, 0);
    const imported = await fetch(`${base}/imports`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${sessions.issue('owner')}`,
        'content-type': 'application/xml',
      },
      body: xml,
    });
    assert.equal(imported.status, 201);
    assert.equal(calls, 1);
    const response = await fetch(`${base}/snapshot`, {
      headers: { authorization: `Bearer ${sessions.issue('owner')}` },
    });
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), expected);
  } finally {
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
});

test('process environment does not forward application credentials', async () => {
  const keys = ['CHEF_SESSION_SECRET', 'GITHUB_CLIENT_SECRET', 'DATABASE_URL'];
  const previous = keys.map((key) => process.env[key]);
  try {
    for (const key of keys) process.env[key] = 'test-secret';
    await fake('ok').correlate(prepared);
  } finally {
    keys.forEach((key, i) => {
      const value = previous[i];
      if (value === undefined) delete process.env[key];
      else process.env[key] = value;
    });
  }
});

test('disconnecting the HTTP client aborts the child and publishes nothing', async () => {
  const projects = new InMemoryProjectRepository();
  const snapshots = new InMemorySnapshotStore();
  const createProject = new CreateProject(projects, identity, clock, {
    token: () => 'disconnect',
  });
  const project = await createProject.execute({
    name: 'Disconnect',
    ownerId: 'owner',
  });
  const sessions = new SessionTokenService({
    secret: 'x'.repeat(32),
    nowMs: () => Date.UTC(2026, 9, 30),
  });
  const engine = fake('hang', { timeoutMs: 5000 });
  let started!: () => void;
  let stopped!: () => void;
  const entered = new Promise<void>((resolve) => {
    started = resolve;
  });
  const exited = new Promise<void>((resolve) => {
    stopped = resolve;
  });
  const server = createChefApi({
    createProject,
    listProjects: new ListProjects(projects),
    importIntoProject: new ImportIntoProject(projects, snapshots, importer, {
      correlate: async (input, signal) => {
        started();
        try {
          return await engine.correlate(input, signal);
        } catch (error) {
          assert.ok(hasCode('CANCELLED')(error));
          throw error;
        } finally {
          stopped();
        }
      },
    }),
    readSnapshot: new ReadProjectSnapshot(projects, snapshots),
    sessions,
    scopePolicy: policy,
    oauth: {
      name: 'fixture',
      authorizeUrl: () => '',
      exchange: async () => {
        throw new Error('unused');
      },
    },
  });
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  assert.ok(address && typeof address !== 'string');
  const cancellation = new AbortController();
  try {
    const request = fetch(
      `http://127.0.0.1:${address.port}/projects/${encodeURIComponent(project.id)}/imports`,
      {
        method: 'POST',
        headers: {
          authorization: `Bearer ${sessions.issue('owner')}`,
          'content-type': 'application/xml',
        },
        body: xml,
        signal: cancellation.signal,
      },
    );
    const rejected = assert.rejects(request, { name: 'AbortError' });
    await entered;
    cancellation.abort();
    await rejected;
    await exited;
    assert.equal(await snapshots.latest(project.id), undefined);
  } finally {
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
});
