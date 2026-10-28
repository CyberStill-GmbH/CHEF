import { Ajv2020 } from 'ajv/dist/2020.js';

import schema from '../../../../docs/contracts/asn-report.schema.json';
import type { AsnReportDto } from './asn-report';

const ajv = new Ajv2020({ allErrors: true, strict: true });
const validate = ajv.compile<AsnReportDto>(schema);

export function decodeAsnReport(value: unknown): AsnReportDto {
  if (!validate(value)) {
    throw new Error('El informe ASN CHEF no cumple su contrato.');
  }
  return value;
}
