import fixture from '../../../../docs/contracts/examples/snapshot.json';
import { describe, expect, it } from 'vitest';
import { FixtureApiClient } from '../api/FixtureApiClient';
import { decodeSnapshot } from '../contracts/decodeSnapshot';

function cloneFixture(): unknown {
  return structuredClone(fixture);
}

function fixtureRecord(): Record<string, unknown> {
  return cloneFixture() as Record<string, unknown>;
}

describe('decodeSnapshot', () => {
  it('accepts the canonical fixture', () => {
    expect(decodeSnapshot(cloneFixture())).toEqual(fixture);
  });

  it('rejects a version other than 1.0.0 with a sanitized error', () => {
    const candidate = fixtureRecord();
    candidate.schemaVersion = '2.0.0';

    expect(() => decodeSnapshot(candidate)).toThrow(
      'Snapshot validation failed.',
    );
  });

  it('rejects an invalid structure', () => {
    const candidate = fixtureRecord();
    delete candidate.assets;

    expect(() => decodeSnapshot(candidate)).toThrow(
      'Snapshot validation failed.',
    );
  });

  it('rejects an unknown observationId', () => {
    const candidate = fixtureRecord();
    const assets = candidate.assets as Array<{ observationIds: string[] }>;
    assets[0]!.observationIds[0] = `observation:${'0'.repeat(64)}`;

    expect(() => decodeSnapshot(candidate)).toThrow(
      'Snapshot references are invalid.',
    );
  });

  it('rejects an unknown evidenceId', () => {
    const candidate = fixtureRecord();
    const observations = candidate.observations as Array<{
      evidenceId: string;
    }>;
    observations[0]!.evidenceId = `evidence:${'0'.repeat(64)}`;

    expect(() => decodeSnapshot(candidate)).toThrow(
      'Snapshot references are invalid.',
    );
  });
});

describe('FixtureApiClient', () => {
  it('rejects when the AbortSignal is already aborted', async () => {
    const controller = new AbortController();
    controller.abort();

    await expect(
      new FixtureApiClient().loadSnapshot({ signal: controller.signal }),
    ).rejects.toMatchObject({ name: 'AbortError' });
  });

  it('preserves every canonical ID without recalculating or modifying it', async () => {
    const snapshot = await new FixtureApiClient().loadSnapshot();
    const actualIds = [
      snapshot.scopePolicy.id,
      snapshot.run.id,
      ...snapshot.evidence.map(({ id }) => id),
      ...snapshot.observations.map(({ id }) => id),
      ...snapshot.assets.map(({ id }) => id),
      ...snapshot.relationships.map(({ id }) => id),
      ...snapshot.paths.map(({ id }) => id),
    ];
    const expectedIds = [
      fixture.scopePolicy.id,
      fixture.run.id,
      ...fixture.evidence.map(({ id }) => id),
      ...fixture.observations.map(({ id }) => id),
      ...fixture.assets.map(({ id }) => id),
      ...fixture.relationships.map(({ id }) => id),
      ...fixture.paths.map(({ id }) => id),
    ];

    expect(actualIds).toEqual(expectedIds);
  });
});
