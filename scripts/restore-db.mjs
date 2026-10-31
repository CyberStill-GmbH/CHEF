#!/usr/bin/env node
import { closeSync, openSync, readFileSync } from 'node:fs';
import { basename } from 'node:path';
import { runDatabaseTool, sha256 } from './db-tools.mjs';

try {
  const file = process.argv[2];
  if (!file || process.argv[3] !== '--yes' || process.argv.length !== 4)
    throw new Error(
      'uso: npm run db:restore -- <archivo.dump> --yes (reemplaza objetos del destino)',
    );
  if (!process.env.DATABASE_URL)
    throw new Error('define DATABASE_URL (no la imprimas en logs)');
  const sidecar = readFileSync(`${file}.sha256`, 'utf8').trim();
  const match = /^([0-9a-f]{64}) {2}(.+)$/.exec(sidecar);
  if (
    !match ||
    match[2] !== basename(file) ||
    match[1] !== (await sha256(file))
  )
    throw new Error(
      'el digest o el nombre no coincide; no se restaura un archivo alterado o incompleto',
    );
  const descriptor = openSync(file, 'r');
  let mode;
  try {
    mode = runDatabaseTool(
      'pg_restore',
      [
        '--clean',
        '--if-exists',
        '--no-owner',
        '--no-privileges',
        '--no-password',
        '--single-transaction',
        '--exit-on-error',
        '--dbname=',
      ],
      { input: descriptor },
    );
  } finally {
    closeSync(descriptor);
  }
  console.log(`restore (${mode}): ${file}`);
} catch (error) {
  console.error(
    `error: ${error instanceof Error ? error.message.replace(/postgres(?:ql)?:\/\/\S+/gi, '[URL omitida]') : 'restauración fallida'}`,
  );
  process.exitCode = 1;
}
