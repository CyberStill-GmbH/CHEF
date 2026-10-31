import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import graphSchema from '../../../../docs/contracts/recon-graph.schema.json';
import schema from '../../../../docs/contracts/recon-graph-progress.schema.json';
import type { ReconGraphProgressDto } from './recon-graph-progress';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
ajv.addSchema(graphSchema);
const validate = ajv.compile<ReconGraphProgressDto>(schema);

export function decodeReconGraphProgress(
  value: unknown,
): ReconGraphProgressDto {
  if (!validate(value)) {
    throw new Error('El avance del grafo CHEF no cumple el contrato.');
  }
  const event = value as ReconGraphProgressDto;
  const counts = [
    event.assets.length,
    event.evidence.length,
    event.relationships.length,
  ];
  if (
    counts.some((count) => count > 100) ||
    (event.phase === 'completed' &&
      Object.values(event.unmappedEventTypes).some(
        (count) => !Number.isSafeInteger(count) || count < 1,
      ))
  ) {
    throw new Error('El lote del grafo CHEF excede sus límites.');
  }
  return event;
}
