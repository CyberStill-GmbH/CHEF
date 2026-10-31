import assert from 'node:assert/strict';
import { Buffer } from 'node:buffer';
import { spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import {
  copyFileSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from 'node:fs';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { SessionTokenService } from '../dist/packages/adapters/src/session-token.js';

const { fetch } = globalThis;

// All writes/restores happen in new disposable resources, never DATABASE_URL.
const suffix = randomUUID().slice(0, 8);
const network = `chef-verify-${suffix}`;
const db = `${network}-db`,
  api = `${network}-api`,
  restoredApi = `${network}-restored`;
const image = process.env.CHEF_VERIFY_IMAGE ?? 'chef-db-operations:local';
const password = 'disposable-test-only';
const sourceUrl = `postgresql://chef:${password}@${db}:5432/chef?schema=public`;
const targetUrl = `postgresql://chef:${password}@${db}:5432/chef_restored?schema=public`;
const directory = resolve('artifacts', 'db-verification', suffix);
mkdirSync(directory, { recursive: true });
const created = [];
let networkCreated = false;

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    windowsHide: true,
    timeout: 120_000,
    ...options,
  });
  if (result.error || result.status !== 0)
    throw new Error(
      `${command} validation step failed; ${result.stderr?.slice(0, 1200) ?? ''}`,
    );
  return result.stdout;
}
function docker(args) {
  return run('docker', args);
}
function sql(database, text) {
  return docker([
    'exec',
    db,
    'psql',
    '-X',
    '-v',
    'ON_ERROR_STOP=1',
    '-U',
    'chef',
    '-d',
    database,
    '-Atc',
    text,
  ]).trim();
}
function fingerprint(database) {
  const data = sql(
    database,
    `SELECT json_build_object(
    'projects',(SELECT json_agg(t ORDER BY id) FROM projects t),
    'memberships',(SELECT json_agg(t ORDER BY project_id,user_id) FROM memberships t),
    'snapshots',(SELECT json_agg(t ORDER BY id) FROM snapshots t))::text`,
  );
  return createHash('sha256').update(data).digest('hex');
}
async function waitForDatabase() {
  for (let i = 0; i < 100; i++) {
    const probe = spawnSync(
      'docker',
      ['exec', db, 'pg_isready', '-U', 'chef', '-d', 'chef'],
      { stdio: 'ignore', windowsHide: true },
    );
    if (probe.status === 0) return;
    await delay(200);
  }
  throw new Error('Temporary PostgreSQL did not become ready');
}
const secret = 'postgres-verification-session-secret';
const sessions = new SessionTokenService({ secret, nowMs: () => Date.now() });
const tokenA = sessions.issue('github:1'),
  tokenB = sessions.issue('github:2');
async function startApi(name, url) {
  docker([
    'run',
    '--detach',
    '--name',
    name,
    '--network',
    network,
    '--publish',
    '127.0.0.1::8787',
    '--env',
    `DATABASE_URL=${url}`,
    '--env',
    `CHEF_SESSION_SECRET=${secret}`,
    '--env',
    'GITHUB_CLIENT_ID=fixture',
    '--env',
    'GITHUB_CLIENT_SECRET=fixture',
    '--env',
    'GITHUB_REDIRECT_URI=http://localhost/callback',
    '--env',
    'HOST=0.0.0.0',
    '--env',
    'CHEF_ALLOWED_GITHUB_USER_IDS=1,2',
    image,
  ]);
  created.push(name);
  const base = apiUrl(name);
  await waitForApi(base);
  return base;
}
function apiUrl(name) {
  const port = JSON.parse(docker(['inspect', name]))[0].NetworkSettings.Ports[
    '8787/tcp'
  ][0].HostPort;
  return `http://127.0.0.1:${port}`;
}
async function waitForApi(base) {
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(`${base}/health`)).ok) return;
    } catch {
      /* starting */
    }
    await delay(200);
  }
  throw new Error('Temporary API did not become ready');
}
async function call(
  base,
  endpoint,
  token,
  method = 'GET',
  body,
  contentType = 'application/json',
) {
  const response = await fetch(`${base}${endpoint}`, {
    method,
    headers: { authorization: `Bearer ${token}`, 'content-type': contentType },
    ...(body === undefined ? {} : { body }),
  });
  return { status: response.status, body: await response.json() };
}
function operation(script, databaseUrl, args, success = true) {
  const result = spawnSync(process.execPath, [script, ...args], {
    encoding: 'utf8',
    windowsHide: true,
    timeout: 120_000,
    env: {
      ...process.env,
      DATABASE_URL: databaseUrl,
      CHEF_DB_TOOLS_MODE: 'docker',
      CHEF_DB_TOOLS_NETWORK: network,
    },
  });
  assert.equal(result.status === 0, success, `${script}: ${result.stderr}`);
  return result;
}

try {
  docker(['network', 'create', network]);
  networkCreated = true;
  docker([
    'run',
    '--detach',
    '--name',
    db,
    '--network',
    network,
    '--env',
    'POSTGRES_USER=chef',
    '--env',
    `POSTGRES_PASSWORD=${password}`,
    '--env',
    'POSTGRES_DB=chef',
    'postgres:16-alpine',
  ]);
  created.push(db);
  await waitForDatabase();
  const migrate = [
    'run',
    '--rm',
    '--network',
    network,
    '--env',
    `DATABASE_URL=${sourceUrl}`,
    '--entrypoint',
    'npm',
    image,
    'run',
    'db:migrate',
  ];
  docker(migrate);
  docker(migrate); // Idempotent migration deployment.
  assert.equal(
    sql(
      'chef',
      "SELECT count(*) FROM pg_tables WHERE schemaname='public' AND tablename IN ('projects','memberships','snapshots')",
    ),
    '3',
  );
  let base = await startApi(api, sourceUrl);
  const a = await call(
    base,
    '/projects',
    tokenA,
    'POST',
    JSON.stringify({ name: 'Verification Alice' }),
  );
  const b = await call(
    base,
    '/projects',
    tokenB,
    'POST',
    JSON.stringify({ name: 'Verification Bob' }),
  );
  assert.equal(a.status, 201);
  assert.equal(b.status, 201);
  const projectId = a.body.project.id;
  const endpoint = `/projects/${encodeURIComponent(projectId)}`;
  const imported = await call(
    base,
    `${endpoint}/imports`,
    tokenA,
    'POST',
    readFileSync('fixtures/nmap/lab.xml'),
    'application/xml',
  );
  assert.equal(imported.status, 201);
  const before = await call(base, `${endpoint}/snapshot`, tokenA);
  assert.equal(before.status, 200);
  assert.equal((await call(base, `${endpoint}/snapshot`, tokenB)).status, 404);
  assert.equal(
    (
      await call(
        base,
        `${endpoint}/imports`,
        tokenB,
        'POST',
        readFileSync('fixtures/nmap/lab.xml'),
        'application/xml',
      )
    ).status,
    404,
  );
  assert.equal((await call(base, '/projects', tokenB)).body.projects.length, 1);
  docker(['restart', api]);
  base = apiUrl(api);
  await waitForApi(base);
  assert.deepEqual(
    (await call(base, `${endpoint}/snapshot`, tokenA)).body,
    before.body,
  );
  const expectedFingerprint = fingerprint('chef');
  operation('scripts/backup-db.mjs', sourceUrl, [directory]);
  const backup = join(
    directory,
    readdirSync(directory).find((name) => name.endsWith('.dump')),
  );
  assert.ok(
    readFileSync(`${backup}.sha256`, 'utf8').startsWith(
      await import('./db-tools.mjs').then((m) => m.sha256(backup)),
    ),
  );
  sql('chef', 'CREATE DATABASE chef_restored');
  operation('scripts/restore-db.mjs', targetUrl, [backup, '--yes']);
  assert.equal(fingerprint('chef_restored'), expectedFingerprint);
  const restoredBase = await startApi(restoredApi, targetUrl);
  assert.deepEqual(
    (await call(restoredBase, `${endpoint}/snapshot`, tokenA)).body,
    before.body,
  );
  assert.equal(
    (await call(restoredBase, `${endpoint}/snapshot`, tokenB)).status,
    404,
  );
  operation('scripts/restore-db.mjs', targetUrl, [backup], false);
  const corruptedDirectory = join(directory, 'corrupted');
  mkdirSync(corruptedDirectory);
  const corrupt = join(corruptedDirectory, backup.split(/[\\/]/).at(-1));
  writeFileSync(
    corrupt,
    Buffer.concat([readFileSync(backup), Buffer.from('tampered')]),
  );
  copyFileSync(`${backup}.sha256`, `${corrupt}.sha256`);
  operation('scripts/restore-db.mjs', targetUrl, [corrupt, '--yes'], false);
  assert.equal(fingerprint('chef_restored'), expectedFingerprint);
  // Force a restore error after DROP has started; the transaction must roll back.
  sql(
    'chef_restored',
    `CREATE FUNCTION reject_restore() RETURNS event_trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'intentional restore test'; END $$;
    CREATE EVENT TRIGGER reject_restore ON ddl_command_start WHEN TAG IN ('CREATE TABLE') EXECUTE FUNCTION reject_restore()`,
  );
  operation('scripts/restore-db.mjs', targetUrl, [backup, '--yes'], false);
  assert.equal(fingerprint('chef_restored'), expectedFingerprint);
  sql(
    'chef_restored',
    'DROP EVENT TRIGGER reject_restore; DROP FUNCTION reject_restore()',
  );
  const report = {
    status: 'ok',
    postgres: '16',
    migration: 'applied twice; three application tables present',
    isolation: 'two users/projects; foreign read/import NOT_FOUND',
    persistence: 'snapshot survived API restart',
    backup: backup.replaceAll('\\', '/'),
    restore: 'same row fingerprint and same API snapshot in separate database',
    negativeControls: [
      'missing --yes',
      'tampered dump',
      'restore error rolls back transaction',
    ],
    oauth: 'synthetic sessions; no live GitHub authentication performed',
  };
  writeFileSync(
    join(directory, 'verification.json'),
    JSON.stringify(report, null, 2) + '\n',
  );
  console.log(JSON.stringify(report, null, 2));
} finally {
  for (const name of created.reverse())
    spawnSync('docker', ['rm', '--force', '--volumes', name], {
      stdio: 'ignore',
      windowsHide: true,
    });
  if (networkCreated)
    spawnSync('docker', ['network', 'rm', network], {
      stdio: 'ignore',
      windowsHide: true,
    });
}
