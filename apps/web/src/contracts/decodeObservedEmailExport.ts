import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import exportSchema from '../../../../docs/contracts/observed-email-export.schema.json';
import type { ObservedEmailExportDto } from './observed-email-export';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validate = ajv.compile<ObservedEmailExportDto>(exportSchema);

export function decodeObservedEmailExport(
  value: unknown,
): ObservedEmailExportDto {
  if (!validate(value)) {
    throw new Error('La exportación de correos CHEF no cumple su contrato.');
  }
  const exported = value as ObservedEmailExportDto;
  if (exported.emailCount !== exported.emails.length) {
    throw new Error(
      'La cantidad de correos de la exportación CHEF no coincide.',
    );
  }
  if (
    new Set(exported.emails.map(({ address }) => address)).size !==
    exported.emailCount
  ) {
    throw new Error('La exportación CHEF contiene direcciones duplicadas.');
  }
  return exported;
}
