import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { InMemoryProjectRepository } from '../packages/adapters/src/memory-projects.js';
import { CreateProject } from '../packages/application/src/create-project.js';
import { ListProjects } from '../packages/application/src/list-projects.js';
import { ChefError } from '../packages/domain/src/errors.js';
import type {
  ClockPort,
  IdentityPort,
  RandomPort,
} from '../packages/domain/src/ports.js';

const clock: ClockPort = { now: () => '2026-10-30T12:00:00.000Z' };
const identity: IdentityPort = {
  digest: () => 'a'.repeat(64),
  id: (namespace, parts) => `${namespace}:${parts.join('-')}`,
};

function counter(): RandomPort {
  let value = 0;
  return {
    token: () => {
      value += 1;
      return `nonce-${value}`;
    },
  };
}

function build() {
  const projects = new InMemoryProjectRepository();
  return {
    projects,
    create: new CreateProject(projects, identity, clock, counter()),
    list: new ListProjects(projects),
  };
}

describe('projects', () => {
  it('creates a project owned by the caller', async () => {
    const { create, projects } = build();
    const project = await create.execute({
      name: '  Auditoría  ACME ',
      ownerId: 'u1',
    });
    assert.equal(project.name, 'Auditoría ACME');
    assert.equal(project.ownerId, 'u1');
    const membership = await projects.findMembership(project.id, 'u1');
    assert.equal(membership?.role, 'owner');
  });

  it('rejects a blank name and control characters', async () => {
    const { create } = build();
    await assert.rejects(
      create.execute({ name: '   ', ownerId: 'u1' }),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'INVALID_INPUT',
    );
    await assert.rejects(
      create.execute({ name: 'a\u0000b', ownerId: 'u1' }),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'INVALID_INPUT',
    );
  });

  it('requires an authenticated owner', async () => {
    const { create } = build();
    await assert.rejects(
      create.execute({ name: 'proyecto', ownerId: '  ' }),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'UNAUTHORIZED',
    );
  });

  it('lists only the projects a user belongs to', async () => {
    const { create, list } = build();
    await create.execute({ name: 'uno', ownerId: 'u1' });
    await create.execute({ name: 'dos', ownerId: 'u2' });
    const mine = await list.execute('u1');
    assert.equal(mine.length, 1);
    assert.equal(mine[0]?.project.name, 'uno');
    assert.equal((await list.execute('u3')).length, 0);
  });

  it('gives two projects their own identity even with the same name', async () => {
    const { create } = build();
    const first = await create.execute({ name: 'mismo', ownerId: 'u1' });
    const second = await create.execute({ name: 'mismo', ownerId: 'u1' });
    assert.notEqual(first.id, second.id);
  });
});
