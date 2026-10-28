import { Ajv2020 } from 'ajv/dist/2020.js';

import targetSeedSchema from '../../../../docs/contracts/target-seed.schema.json';
import type { TargetSeedDto } from './target-seed';

const ajv = new Ajv2020({ allErrors: true, strict: true });
const validateTargetSeed = ajv.compile<TargetSeedDto>(targetSeedSchema);

export function decodeTargetSeed(value: unknown): TargetSeedDto {
  if (!validateTargetSeed(value)) {
    throw new Error('El objetivo recibido no cumple el contrato CHEF.');
  }
  return value;
}
