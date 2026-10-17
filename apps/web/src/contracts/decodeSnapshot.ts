import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import snapshotSchema from '../../../../docs/contracts/snapshot.schema.json';
import type { SnapshotDto } from './snapshot';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validateSnapshot = ajv.compile<SnapshotDto>(snapshotSchema);

const SCHEMA_ERROR = 'Snapshot validation failed.';
const REFERENCE_ERROR = 'Snapshot references are invalid.';

function hasUniqueIds(items: ReadonlyArray<{ id: string }>): boolean {
  return new Set(items.map(({ id }) => id)).size === items.length;
}

function referencesOnly(
  ids: readonly string[],
  available: ReadonlySet<string>,
): boolean {
  return ids.every((id) => available.has(id));
}

function hasReferentialIntegrity(snapshot: SnapshotDto): boolean {
  if (
    snapshot.schemaVersion !== '1.0.0' ||
    snapshot.scopePolicy.schemaVersion !== '1.0.0' ||
    snapshot.run.scopePolicyId !== snapshot.scopePolicy.id ||
    !hasUniqueIds(snapshot.evidence) ||
    !hasUniqueIds(snapshot.observations) ||
    !hasUniqueIds(snapshot.assets) ||
    !hasUniqueIds(snapshot.relationships) ||
    !hasUniqueIds(snapshot.paths)
  ) {
    return false;
  }

  const evidenceIds = new Set(snapshot.evidence.map(({ id }) => id));
  const observationIds = new Set(snapshot.observations.map(({ id }) => id));
  const assetIds = new Set(snapshot.assets.map(({ id }) => id));
  const relationshipIds = new Set(snapshot.relationships.map(({ id }) => id));

  if (
    snapshot.evidence.some(({ runId }) => runId !== snapshot.run.id) ||
    snapshot.observations.some(
      ({ evidenceId, runId }) =>
        !evidenceIds.has(evidenceId) || runId !== snapshot.run.id,
    ) ||
    snapshot.assets.some(
      ({ observationIds: references }) =>
        !referencesOnly(references, observationIds),
    ) ||
    snapshot.relationships.some(
      ({ from, to, observationIds: references }) =>
        from === to ||
        !assetIds.has(from) ||
        !assetIds.has(to) ||
        !referencesOnly(references, observationIds),
    )
  ) {
    return false;
  }

  const relationshipsById = new Map(
    snapshot.relationships.map((relationship) => [
      relationship.id,
      relationship,
    ]),
  );

  return snapshot.paths.every(
    ({ assetIds: pathAssets, relationshipIds: pathRelationships }) => {
      if (
        !referencesOnly(pathAssets, assetIds) ||
        !referencesOnly(pathRelationships, relationshipIds)
      ) {
        return false;
      }

      const relationship = relationshipsById.get(pathRelationships[0] ?? '');
      return (
        relationship !== undefined &&
        pathAssets.includes(relationship.from) &&
        pathAssets.includes(relationship.to)
      );
    },
  );
}

export function decodeSnapshot(value: unknown): SnapshotDto {
  if (!validateSnapshot(value)) {
    throw new Error(SCHEMA_ERROR);
  }

  if (!hasReferentialIntegrity(value)) {
    throw new Error(REFERENCE_ERROR);
  }

  return value;
}
