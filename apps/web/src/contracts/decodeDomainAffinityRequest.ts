import { Ajv2020 } from 'ajv/dist/2020.js';

import schema from '../../../../docs/contracts/domain-affinity-request.schema.json';
import type { DomainAffinityRequestDto } from './domain-affinity';

const ajv = new Ajv2020({ allErrors: true, strict: true });
const validate = ajv.compile<DomainAffinityRequestDto>(schema);

export function decodeDomainAffinityRequest(
  value: unknown,
): DomainAffinityRequestDto {
  if (!validate(value)) {
    throw new Error('La solicitud de afinidad CHEF no cumple su contrato.');
  }
  return value;
}
