import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

// Disposable, network-isolated container; OAuth and DB are not exercised here.
const source = `
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFileSync } from 'node:fs';
import { setTimeout as delay } from 'node:timers/promises';
import { SessionTokenService } from './dist/packages/adapters/src/session-token.js';
const xml = readFileSync(0);
const secret = 'offline-container-smoke-test-secret';
const api = spawn(process.execPath, ['dist/apps/api/src/main.js'], {
  env: { ...process.env, CHEF_SESSION_SECRET: secret, GITHUB_CLIENT_ID: 'offline',
    GITHUB_CLIENT_SECRET: 'offline', GITHUB_REDIRECT_URI: 'http://localhost/callback' },
  stdio: ['ignore', 'ignore', 'inherit'],
});
const closed = once(api, 'close');
const base = 'http://127.0.0.1:8787';
try {
  let ready = false;
  for (let i = 0; i < 100; i++) {
    try { ready = (await fetch(base + '/health')).ok; } catch {}
    if (ready) break;
    if (api.exitCode !== null) throw Error('API exited during startup');
    await delay(50);
  }
  assert.ok(ready, 'API did not become ready');
  const sessions = new SessionTokenService({ secret, nowMs: () => Date.now() });
  const authorization = 'Bearer ' + sessions.issue('smoke-owner');
  const created = await fetch(base + '/projects', { method: 'POST',
    headers: { authorization, 'content-type': 'application/json' }, body: JSON.stringify({ name: 'Offline smoke' }) });
  assert.equal(created.status, 201);
  const { project } = await created.json();
  const endpoint = base + '/projects/' + encodeURIComponent(project.id);
  const imported = await fetch(endpoint + '/imports', { method: 'POST',
    headers: { authorization, 'content-type': 'application/xml' }, body: xml });
  assert.equal(imported.status, 201, JSON.stringify(await imported.json()));
  const response = await fetch(endpoint + '/snapshot', { headers: { authorization } });
  assert.equal(response.status, 200);
  const snapshot = await response.json();
  assert.ok(snapshot.assets.length > 0);
  assert.ok(snapshot.paths.length > 0);
  assert.ok(snapshot.paths.every(p => p.isConfirmedVulnerability === false));
  const denied = await fetch(endpoint + '/snapshot', { headers: { authorization: 'Bearer ' + sessions.issue('outsider') } });
  assert.equal(denied.status, 404);
  console.log(JSON.stringify({ status: 'ok', runtime: 'Linux container, Node + packaged Go',
    assets: snapshot.assets.length, paths: snapshot.paths.length, outsiderStatus: denied.status }));
} finally {
  api.kill('SIGTERM');
  await closed;
}
`;
const result = execFileSync(
  'docker',
  [
    'run',
    '--rm',
    '-i',
    '--network',
    'none',
    '--entrypoint',
    'node',
    process.argv[2] ?? 'chef-go-integration:local',
    '--input-type=module',
    '-e',
    source,
  ],
  {
    input: readFileSync('fixtures/nmap/lab.xml'),
    encoding: 'utf8',
    timeout: 30_000,
  },
);
process.stdout.write(result);
