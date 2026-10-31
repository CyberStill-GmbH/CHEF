import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { test } from 'node:test';

function script(name: string, args: string[], env: NodeJS.ProcessEnv = {}) {
  const clean = { ...process.env, ...env };
  delete clean['PGPASSWORD'];
  return spawnSync(process.execPath, [`scripts/${name}.mjs`, ...args], {
    env: clean,
    encoding: 'utf8',
  });
}

test('backup requires an explicit connection without printing it', () => {
  const result = script('backup-db', [], { DATABASE_URL: '' });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /define DATABASE_URL/);
});

test('libpq configuration removes Prisma schema and decodes credentials', () => {
  const source = `import assert from 'node:assert/strict';
    import { connectionEnvironment } from './scripts/db-tools.mjs';
    const c = connectionEnvironment('postgresql://test:p%3Aa%5Css@127.0.0.1:5555/chef?schema=public&sslmode=require');
    assert.equal(c.env.PGHOST, '127.0.0.1'); assert.equal(c.env.PGPORT, '5555');
    assert.equal(c.env.PGDATABASE, 'chef'); assert.equal(c.env.PGSSLMODE, 'require');
    assert.equal(c.password, 'p:a\\\\ss');
    assert.throws(() => connectionEnvironment('https://example.invalid/chef'));
    assert.throws(() => connectionEnvironment('postgresql://u:p@localhost/chef?unsupported=1'));
    assert.throws(() => connectionEnvironment('postgresql://u:p%0A@localhost/chef'));
  `;
  const result = spawnSync(
    process.execPath,
    ['--input-type=module', '-e', source],
    { encoding: 'utf8' },
  );
  assert.equal(result.status, 0, result.stderr);
});

test('restore refuses missing confirmation and altered archives before connecting', () => {
  const directory = mkdtempSync(join(tmpdir(), 'chef-backup-test-'));
  try {
    const file = join(directory, 'archive with spaces.dump');
    const data = Buffer.from('synthetic-archive');
    writeFileSync(file, data);
    const digest = createHash('sha256').update(data).digest('hex');
    writeFileSync(`${file}.sha256`, `${digest}  ${basename(file)}\n`);
    const env = {
      DATABASE_URL:
        'postgresql://chef:secret-marker@127.0.0.1:1/chef?schema=public',
      CHEF_DB_TOOLS_MODE: 'invalid',
    };
    const unconfirmed = script('restore-db', [file], env);
    assert.notEqual(unconfirmed.status, 0);
    assert.match(unconfirmed.stderr, /--yes/);
    writeFileSync(file, 'altered');
    const altered = script('restore-db', [file, '--yes'], env);
    assert.notEqual(altered.status, 0);
    assert.match(altered.stderr, /digest/);
    assert.ok(!altered.stderr.includes('secret-marker'));
    writeFileSync(file, data);
    writeFileSync(`${file}.sha256`, `${digest}  another.dump\n`);
    assert.match(script('restore-db', [file, '--yes'], env).stderr, /digest/);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('failed backup leaves no finished archive or partial file', () => {
  const directory = mkdtempSync(join(tmpdir(), 'chef-backup-failure-'));
  try {
    const result = script('backup-db', [directory], {
      DATABASE_URL: 'postgresql://chef:secret-marker@localhost/chef',
      CHEF_DB_TOOLS_MODE: 'invalid',
    });
    assert.notEqual(result.status, 0);
    assert.deepEqual(readdirSync(directory), []);
    assert.ok(!result.stderr.includes('secret-marker'));
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('production startup refuses memory persistence and an empty allowlist', () => {
  for (const [databaseUrl, allowedIds, expected] of [
    ['', '1', 'DATABASE_URL'],
    [
      'postgresql://chef:synthetic@localhost/chef',
      '',
      'CHEF_ALLOWED_GITHUB_USER_IDS',
    ],
    [
      'postgresql://chef:synthetic@localhost/chef',
      ',',
      'CHEF_ALLOWED_GITHUB_USER_IDS',
    ],
  ]) {
    const result = spawnSync(process.execPath, ['dist/apps/api/src/main.js'], {
      encoding: 'utf8',
      timeout: 10_000,
      env: {
        ...process.env,
        NODE_ENV: 'production',
        DATABASE_URL: databaseUrl,
        CHEF_ALLOWED_GITHUB_USER_IDS: allowedIds,
      },
    });
    assert.notEqual(result.status, 0);
    assert.ok(result.stderr.includes(expected ?? ''));
    assert.ok(!result.stderr.includes('postgresql://'));
  }
});
