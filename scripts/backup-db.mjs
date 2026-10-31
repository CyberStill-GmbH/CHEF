#!/usr/bin/env node
import { randomUUID } from 'node:crypto';
import {
  closeSync,
  mkdirSync,
  openSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { basename, join } from 'node:path';
import { runDatabaseTool, sha256 } from './db-tools.mjs';

let temporary;
try {
  const directory = process.argv[2] ?? join('artifacts', 'backup');
  if (process.argv.length > 3)
    throw new Error('uso: npm run db:backup -- [directorio]');
  if (!process.env.DATABASE_URL)
    throw new Error('define DATABASE_URL (no la imprimas en logs)');
  mkdirSync(directory, { recursive: true });
  const stamp = new Date().toISOString().replace(/[-:.]/g, '');
  const file = join(
    directory,
    `chef-${stamp}-${randomUUID().slice(0, 8)}.dump`,
  );
  temporary = `${file}.partial`;
  const descriptor = openSync(temporary, 'wx', 0o600);
  let mode;
  try {
    mode = runDatabaseTool(
      'pg_dump',
      ['--format=custom', '--no-owner', '--no-privileges', '--no-password'],
      { output: descriptor },
    );
  } finally {
    closeSync(descriptor);
  }
  const digest = await sha256(temporary);
  renameSync(temporary, file);
  temporary = undefined;
  writeFileSync(`${file}.sha256`, `${digest}  ${basename(file)}\n`, {
    mode: 0o600,
    flag: 'wx',
  });
  console.log(`backup (${mode}): ${file}`);
  console.log(`sha256: ${digest}`);
} catch (error) {
  if (temporary) rmSync(temporary, { force: true });
  console.error(
    `error: ${error instanceof Error ? error.message.replace(/postgres(?:ql)?:\/\/\S+/gi, '[URL omitida]') : 'backup fallido'}`,
  );
  process.exitCode = 1;
}
