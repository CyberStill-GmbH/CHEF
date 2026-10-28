import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import exportSchema from '../../../../docs/contracts/observed-domain-export.schema.json';
import type { ObservedDomainExportDto } from './observed-domain-export';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validate = ajv.compile<ObservedDomainExportDto>(exportSchema);

export function decodeObservedDomainExport(
  value: unknown,
): ObservedDomainExportDto {
  if (!validate(value)) {
    throw new Error('La exportación de dominios observados no es válida.');
  }
  return value;
}
