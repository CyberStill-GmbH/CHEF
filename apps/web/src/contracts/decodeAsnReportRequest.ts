import { Ajv2020 } from 'ajv/dist/2020.js';

import schema from '../../../../docs/contracts/asn-report-request.schema.json';
import type { AsnReportRequestDto } from './asn-report';

const ajv = new Ajv2020({ allErrors: true, strict: true });
const validate = ajv.compile<AsnReportRequestDto>(schema);

export function decodeAsnReportRequest(value: unknown): AsnReportRequestDto {
  if (!validate(value)) {
    throw new Error('La solicitud de informe ASN CHEF no cumple su contrato.');
  }
  return value;
}
