import { Ajv2020 } from 'ajv/dist/2020.js';

import exportSchema from '../../../../docs/contracts/evidence-csv-export.schema.json';
import type { EvidenceCsvExportDto } from './evidence-csv-export';

const ajv = new Ajv2020({ allErrors: true, strict: true });
const validate = ajv.compile<EvidenceCsvExportDto>(exportSchema);

export function decodeEvidenceCsvExport(value: unknown): EvidenceCsvExportDto {
  if (!validate(value)) {
    throw new Error('La exportación CSV de evidencia no es válida.');
  }
  return value;
}
