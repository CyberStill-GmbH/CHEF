import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';
test('CLI emits valid JSON for fixture and no partial result on hostile input', () => {
  const success = spawnSync(
    process.execPath,
    [
      'dist/apps/cli/src/main.js',
      'fixtures/nmap/lab.xml',
      'fixtures/scope/lab.json',
    ],
    { encoding: 'utf8' },
  );
  assert.equal(success.status, 0, success.stderr);
  assert.equal(JSON.parse(success.stdout).assets.length, 4);
  const hostile = spawnSync(
    process.execPath,
    [
      'dist/apps/cli/src/main.js',
      'fixtures/nmap/hostile-xxe.xml',
      'fixtures/scope/lab.json',
    ],
    { encoding: 'utf8' },
  );
  assert.equal(hostile.status, 1);
  assert.equal(hostile.stdout, '');
  assert.equal(JSON.parse(hostile.stderr).code, 'INVALID_INPUT');
  assert.ok(!hostile.stderr.includes('file:///'));
});
