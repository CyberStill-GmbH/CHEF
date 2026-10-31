#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream, existsSync, readFileSync } from 'node:fs';

/**
 * Restauración de un backup lógico de CHEF.
 * Verificado en seco: argumentos, confirmación explícita e integridad SHA-256.
 * La ejecución real queda pendiente hasta tener PostgreSQL y pg_restore.
 *
 * Uso: DATABASE_URL=... node scripts/restore-db.mjs <archivo.dump> --yes
 */

function fail(message, code) {
  console.error(`error: ${message}`);
  process.exit(code);
}

const file = process.argv[2];
const confirm = process.argv[3];

if (file === undefined || !existsSync(file))
  fail('uso: node scripts/restore-db.mjs <archivo.dump> --yes', 2);
if (confirm !== '--yes')
  fail('la restauración reemplaza objetos; repite con --yes', 2);

const databaseUrl = process.env['DATABASE_URL'];
if (databaseUrl === undefined || databaseUrl.trim() === '')
  fail('define DATABASE_URL (no la imprimas en logs)', 2);

const sidecar = `${file}.sha256`;
if (!existsSync(sidecar))
  fail(`falta ${sidecar}; no se restaura sin verificar integridad`, 3);

const expected = readFileSync(sidecar, 'utf8').trim().split(/\s+/)[0];
const actual = await new Promise((resolve, reject) => {
  const hash = createHash('sha256');
  createReadStream(file)
    .on('error', reject)
    .on('data', (chunk) => hash.update(chunk))
    .on('end', () => resolve(hash.digest('hex')));
});

if (expected === undefined || expected !== actual)
  fail('el digest no coincide; archivo alterado o incompleto', 3);

const restore = spawnSync(
  'pg_restore',
  [
    '--clean',
    '--if-exists',
    '--no-owner',
    '--no-privileges',
    `--dbname=${databaseUrl}`,
    file,
  ],
  { stdio: 'inherit' },
);

if (restore.error !== undefined) {
  if (restore.error.code === 'ENOENT')
    fail('pg_restore no está instalado o no está en PATH', 3);
  fail(restore.error.message, 3);
}
if (restore.status !== 0)
  fail(`pg_restore terminó con código ${restore.status}`, 3);

console.log(`restore: ${file}`);
