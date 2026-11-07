#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream, mkdirSync } from 'node:fs';
import { basename, join } from 'node:path';

/**
 * Backup lógico de PostgreSQL para CHEF.
 * Verificado en seco: validación de configuración y manifiesto de integridad.
 * La ejecución real queda pendiente hasta tener PostgreSQL y pg_dump.
 *
 * Uso: DATABASE_URL=... node scripts/backup-db.mjs [directorio]
 */

function fail(message, code) {
  console.error(`error: ${message}`);
  process.exit(code);
}

const databaseUrl = process.env['DATABASE_URL'];
if (databaseUrl === undefined || databaseUrl.trim() === '')
  fail('define DATABASE_URL (no la imprimas en logs)', 2);

const outDir = process.argv[2] ?? join('artifacts', 'backup');
mkdirSync(outDir, { recursive: true });

const stamp = new Date()
  .toISOString()
  .replace(/[-:]/g, '')
  .replace(/\..+/, 'Z');
const file = join(outDir, `chef-${stamp}.dump`);

const dump = spawnSync(
  'pg_dump',
  [
    '--format=custom',
    '--no-owner',
    '--no-privileges',
    `--file=${file}`,
    `--dbname=${databaseUrl}`,
  ],
  { stdio: 'inherit' },
);

if (dump.error !== undefined) {
  if (dump.error.code === 'ENOENT')
    fail('pg_dump no está instalado o no está en PATH', 3);
  fail(dump.error.message, 3);
}
if (dump.status !== 0) fail(`pg_dump terminó con código ${dump.status}`, 3);

const digest = await new Promise((resolve, reject) => {
  const hash = createHash('sha256');
  createReadStream(file)
    .on('error', reject)
    .on('data', (chunk) => hash.update(chunk))
    .on('end', () => resolve(hash.digest('hex')));
});

const { writeFileSync } = await import('node:fs');
writeFileSync(`${file}.sha256`, `${digest}  ${basename(file)}\n`, 'utf8');

console.log(`backup: ${file}`);
console.log(`sha256: ${digest}`);
