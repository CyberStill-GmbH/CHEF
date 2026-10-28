import { Ajv2020 } from 'ajv/dist/2020.js';

import configSchema from '../../../../docs/contracts/chef-config.schema.json';
import type { ChefConfigDto } from './chef-config';

const ajv = new Ajv2020({ allErrors: true, strict: true });
const validateConfig = ajv.compile<ChefConfigDto>(configSchema);

export function decodeChefConfig(value: unknown): ChefConfigDto {
  if (!validateConfig(value)) {
    throw new Error('La configuración CHEF recibida no cumple su contrato.');
  }
  return value;
}
