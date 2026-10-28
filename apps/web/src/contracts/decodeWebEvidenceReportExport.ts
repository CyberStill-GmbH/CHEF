import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import exportSchema from '../../../../docs/contracts/web-evidence-report-export.schema.json';
import type { WebEvidenceReportExportDto } from './web-evidence-report-export';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validate = ajv.compile<WebEvidenceReportExportDto>(exportSchema);

export function decodeWebEvidenceReportExport(
  value: unknown,
): WebEvidenceReportExportDto {
  if (!validate(value)) {
    throw new Error('El informe web CHEF no cumple su contrato.');
  }
  return value;
}
