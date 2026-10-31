#!/usr/bin/env node
// Prueba de aislamiento multiusuario contra PostgreSQL real, usando el adaptador
// Prisma de verdad (PrismaProjectRepository + PrismaSnapshotStore) y el caso de
// uso ReadProjectSnapshot. Un usuario sin membresía debe recibir NOT_FOUND.
//
// Uso: DATABASE_URL=postgresql://... node scripts/verify-prisma-isolation.mjs
import { readFileSync } from 'node:fs';

import { PrismaClient } from '@prisma/client';

import {
  PrismaProjectRepository,
  PrismaSnapshotStore,
} from '../dist/packages/adapters/src/prisma-projects.js';
import { ReadProjectSnapshot } from '../dist/packages/application/src/read-project-snapshot.js';

const snapshot = JSON.parse(
  readFileSync(
    new URL('../docs/contracts/examples/snapshot.json', import.meta.url),
    'utf8',
  ),
);

const now = () => new Date().toISOString();
const prisma = new PrismaClient();
const projects = new PrismaProjectRepository(prisma);
const snapshots = new PrismaSnapshotStore(prisma, { now });
const read = new ReadProjectSnapshot(projects, snapshots);

const suffix = Date.now().toString(36);
const projectId = `project:iso-${suffix}`;
const owner = `user:owner-${suffix}`;
const stranger = `user:stranger-${suffix}`;

let failures = 0;
function check(label, ok) {
  console.log(`${ok ? 'PASS' : 'FAIL'}: ${label}`);
  if (!ok) failures += 1;
}

try {
  await projects.saveProject({
    id: projectId,
    name: `Isolation ${suffix}`,
    ownerId: owner,
    createdAt: now(),
  });
  await projects.saveMembership({ projectId, userId: owner, role: 'owner' });
  await snapshots.save(projectId, snapshot);

  // 1) El dueño sí puede leer su snapshot.
  const got = await read.execute(projectId, owner);
  check(
    'el dueño lee el snapshot de su proyecto',
    got.schemaVersion === '1.0.0',
  );

  // 2) Un usuario ajeno (sin membresía) recibe NOT_FOUND, no el snapshot.
  let thrown;
  try {
    await read.execute(projectId, stranger);
  } catch (error) {
    thrown = error;
  }
  check(
    'un usuario ajeno recibe NOT_FOUND (no se filtra el proyecto)',
    thrown?.name === 'ChefError' && thrown?.code === 'NOT_FOUND',
  );

  // 3) findMembership del ajeno devuelve undefined en la base real.
  const strangerMembership = await projects.findMembership(projectId, stranger);
  check(
    'findMembership del ajeno es undefined',
    strangerMembership === undefined,
  );
} finally {
  // Limpieza: borra el proyecto; el ON DELETE CASCADE arrastra membresías y snapshots.
  await prisma.project
    .delete({ where: { id: projectId } })
    .catch(() => undefined);
  await prisma.$disconnect();
}

if (failures > 0) {
  console.error(`\n${failures} comprobación(es) fallaron`);
  process.exit(1);
}
console.log(
  '\nOK: aislamiento multiusuario verificado contra PostgreSQL real.',
);
