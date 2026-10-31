import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import schema from '../../../../docs/contracts/urlscan-import.schema.json';
import type { UrlScanImportSummaryDto } from './urlscan-import';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validate = ajv.compile<UrlScanImportSummaryDto>(schema);

export function decodeUrlScanImport(value: unknown): UrlScanImportSummaryDto {
  if (!validate(value)) {
    throw new Error(
      'El resumen de importación urlscan no cumple el contrato CHEF.',
    );
  }
  return value;
}
