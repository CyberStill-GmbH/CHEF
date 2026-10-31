import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import schema from '../../../../docs/contracts/recon-graph.schema.json';
import type { ReconGraphDto } from './recon-graph';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validateGraph = ajv.compile<ReconGraphDto>(schema);

function hasUniqueIds(items: ReadonlyArray<{ id: string }>): boolean {
  return new Set(items.map(({ id }) => id)).size === items.length;
}

function graphReferencesAreValid(graph: ReconGraphDto): boolean {
  if (
    !hasUniqueIds(graph.assets) ||
    !hasUniqueIds(graph.evidence) ||
    !hasUniqueIds(graph.relationships)
  ) {
    return false;
  }

  const assetIds = new Set(graph.assets.map(({ id }) => id));
  const evidenceIds = new Set(graph.evidence.map(({ id }) => id));
  const hasNewQualifications = graph.assets.some(
    ({ kind, qualification }) =>
      (kind === 'domain' &&
        (qualification === 'hint' || qualification === 'unresolved')) ||
      (kind === 'ip_address' && qualification === 'hint'),
  );
  const hasOrganizations = graph.assets.some(
    ({ kind }) => kind === 'organization',
  );

  return (
    (graph.schemaVersion === '1.3.0' ||
      graph.schemaVersion === '1.4.0' ||
      !hasNewQualifications) &&
    (graph.schemaVersion === '1.4.0' || !hasOrganizations) &&
    graph.evidence.every(
      ({ parentEvidenceId }) =>
        parentEvidenceId === null || evidenceIds.has(parentEvidenceId),
    ) &&
    graph.assets.every(({ evidenceIds: references }) =>
      references.every((id) => evidenceIds.has(id)),
    ) &&
    graph.relationships.every(
      ({ fromAssetId, toAssetId, evidenceIds: references }) =>
        fromAssetId !== toAssetId &&
        assetIds.has(fromAssetId) &&
        assetIds.has(toAssetId) &&
        references.every((id) => evidenceIds.has(id)),
    )
  );
}

export function decodeReconGraph(value: unknown): ReconGraphDto {
  if (!validateGraph(value)) {
    throw new Error('El grafo de reconocimiento CHEF no es válido.');
  }
  if (!graphReferencesAreValid(value)) {
    throw new Error(
      'El grafo de reconocimiento CHEF tiene referencias inválidas.',
    );
  }
  return value;
}
