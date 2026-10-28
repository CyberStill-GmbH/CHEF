import { Ajv2020 } from 'ajv/dist/2020.js';

import schema from '../../../../docs/contracts/domain-affinity.schema.json';
import type { DomainAffinityReportDto } from './domain-affinity';

const ajv = new Ajv2020({ allErrors: true, strict: true });
const validate = ajv.compile<DomainAffinityReportDto>(schema);

export function decodeDomainAffinity(value: unknown): DomainAffinityReportDto {
  if (!validate(value)) {
    throw new Error('El informe de afinidad CHEF no cumple su contrato.');
  }
  return value;
}
