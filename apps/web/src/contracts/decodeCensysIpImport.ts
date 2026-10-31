import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import schema from '../../../../docs/contracts/censys-ip-import.schema.json';
import type { CensysIpImportSummaryDto } from './censys-ip-import';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validate = ajv.compile<CensysIpImportSummaryDto>(schema);

export function decodeCensysIpImport(value: unknown): CensysIpImportSummaryDto {
  if (!validate(value)) {
    throw new Error(
      'El resumen de importación Censys no cumple el contrato CHEF.',
    );
  }
  return value;
}
