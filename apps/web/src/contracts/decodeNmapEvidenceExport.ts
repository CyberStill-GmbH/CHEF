import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import exportSchema from '../../../../docs/contracts/nmap-evidence-export.schema.json';
import type { NmapEvidenceExportDto } from './nmap-evidence-export';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validate = ajv.compile<NmapEvidenceExportDto>(exportSchema);

export function decodeNmapEvidenceExport(
  value: unknown,
): NmapEvidenceExportDto {
  if (!validate(value)) {
    throw new Error('La exportación Nmap XML de CHEF no es válida.');
  }
  return value;
}
