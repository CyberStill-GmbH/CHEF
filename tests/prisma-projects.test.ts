import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import {
  PrismaProjectRepository,
  PrismaSnapshotStore,
  type PrismaLikeClient,
  type PrismaMembershipRow,
  type PrismaProjectRow,
  type PrismaSnapshotRow,
} from '../packages/adapters/src/prisma-projects.js';
import { ChefError } from '../packages/domain/src/errors.js';
import type { Snapshot } from '../packages/domain/src/model.js';

function uniqueViolation(): Error {
  return Object.assign(new Error('Unique constraint failed'), {
    code: 'P2002',
  });
}

class FakePrismaClient implements PrismaLikeClient {
  readonly projects = new Map<string, PrismaProjectRow>();
  readonly memberships = new Map<string, PrismaMembershipRow>();
  readonly snapshots: Array<
    PrismaSnapshotRow & { projectId: string; createdAt: Date }
  > = [];

  readonly project = {
    create: async ({ data }: { data: PrismaProjectRow }) => {
      if (this.projects.has(data.id)) throw uniqueViolation();
      const row: PrismaProjectRow = { ...data };
      this.projects.set(row.id, row);
      return { ...row };
    },
    findUnique: async ({ where }: { where: { id: string } }) => {
      const row = this.projects.get(where.id);
      return row === undefined ? null : { ...row };
    },
  };

  readonly membership = {
    upsert: async ({
      create,
    }: {
      create: { projectId: string; userId: string; role: string };
    }) => {
      const key = `${create.projectId}\u0000${create.userId}`;
      this.memberships.set(key, { ...create });
      return { ...create };
    },
    findUnique: async ({
      where,
    }: {
      where: { projectId_userId: { projectId: string; userId: string } };
    }) => {
      const key = `${where.projectId_userId.projectId}\u0000${where.projectId_userId.userId}`;
      const row = this.memberships.get(key);
      return row === undefined ? null : { ...row };
    },
    findMany: async ({ where }: { where: { userId: string } }) => {
      const rows: PrismaMembershipRow[] = [];
      for (const row of this.memberships.values()) {
        if (row.userId !== where.userId) continue;
        const project = this.projects.get(row.projectId);
        rows.push(
          project === undefined
            ? { ...row }
            : { ...row, project: { ...project } },
        );
      }
      return rows;
    },
  };

  readonly snapshot = {
    create: async ({
      data,
    }: {
      data: {
        projectId: string;
        runId: string;
        payload: unknown;
        createdAt: Date;
      };
    }) => {
      this.snapshots.push({
        projectId: data.projectId,
        payload: structuredClone(data.payload),
        createdAt: data.createdAt,
      });
      return {};
    },
    findFirst: async ({ where }: { where: { projectId: string } }) => {
      const rows = this.snapshots
        .filter((row) => row.projectId === where.projectId)
        .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());
      const latest = rows[0];
      return latest === undefined ? null : { payload: latest.payload };
    },
  };
}

const project = {
  id: 'project:1',
  name: 'ACME',
  ownerId: 'u1',
  createdAt: '2026-10-30T00:00:00.000Z',
};

function snapshot(runId: string): Snapshot {
  return {
    schemaVersion: '1.0.0',
    scopePolicy: {
      schemaVersion: '1.0.0',
      id: 'scope',
      allowedAddresses: ['127.0.0.1'],
      excludedAddresses: [],
      allowedPorts: [80],
      allowedProtocols: ['tcp'],
      maxInputBytes: 1,
      maxObservations: 1,
    },
    run: {
      id: runId,
      scopePolicyId: 'scope',
      scopeDigest: 'digest',
      state: 'completed',
      startedAt: '2026-10-30T00:00:00.000Z',
      finishedAt: '2026-10-30T00:00:00.000Z',
    },
    evidence: [],
    observations: [],
    assets: [],
    relationships: [],
    paths: [],
  };
}

describe('Prisma adapters (cliente falso)', () => {
  it('guarda y recupera un proyecto con su membresía', async () => {
    const client = new FakePrismaClient();
    const repository = new PrismaProjectRepository(client);
    await repository.saveProject(project);
    await repository.saveMembership({
      projectId: project.id,
      userId: 'u1',
      role: 'owner',
    });

    assert.deepEqual(await repository.findProject(project.id), project);
    const membership = await repository.findMembership(project.id, 'u1');
    assert.equal(membership?.role, 'owner');
    assert.equal(await repository.findMembership(project.id, 'u2'), undefined);
  });

  it('traduce una clave duplicada a CONFLICT', async () => {
    const repository = new PrismaProjectRepository(new FakePrismaClient());
    await repository.saveProject(project);
    await assert.rejects(
      repository.saveProject(project),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'CONFLICT',
    );
  });

  it('lista solo los proyectos del usuario y actualiza el rol', async () => {
    const client = new FakePrismaClient();
    const repository = new PrismaProjectRepository(client);
    await repository.saveProject(project);
    await repository.saveProject({ ...project, id: 'project:2', name: 'Otro' });
    await repository.saveMembership({
      projectId: project.id,
      userId: 'u1',
      role: 'member',
    });
    await repository.saveMembership({
      projectId: 'project:2',
      userId: 'u2',
      role: 'owner',
    });

    const mine = await repository.listProjectsForUser('u1');
    assert.deepEqual(
      mine.map((item) => item.project.id),
      [project.id],
    );
    assert.equal(mine[0]?.role, 'member');

    await repository.saveMembership({
      projectId: project.id,
      userId: 'u1',
      role: 'owner',
    });
    assert.equal(
      (await repository.findMembership(project.id, 'u1'))?.role,
      'owner',
    );
  });

  it('devuelve el snapshot más reciente y lo aísla del original', async () => {
    const client = new FakePrismaClient();
    let now = Date.parse('2026-10-30T00:00:00.000Z');
    const store = new PrismaSnapshotStore(client, {
      now: () => new Date(now).toISOString(),
    });

    const first = snapshot('run:1');
    await store.save(project.id, first);
    now += 1000;
    await store.save(project.id, snapshot('run:2'));

    const latest = await store.latest(project.id);
    assert.equal(latest?.run.id, 'run:2');
    assert.notEqual(latest, client.snapshots[1]?.payload);
    assert.equal(await store.latest('project:missing'), undefined);
  });

  it('rechaza un payload almacenado que no es Snapshot 1.0.0', async () => {
    const client = new FakePrismaClient();
    const store = new PrismaSnapshotStore(client, {
      now: () => '2026-10-30T00:00:00.000Z',
    });
    client.snapshots.push({
      projectId: project.id,
      payload: { schemaVersion: '2.0.0' },
      createdAt: new Date('2026-10-30T00:00:00.000Z'),
    });
    await assert.rejects(store.latest(project.id), /Snapshot 1\.0\.0/);
  });
});
