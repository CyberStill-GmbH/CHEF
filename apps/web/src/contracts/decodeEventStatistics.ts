import { Ajv2020 } from 'ajv/dist/2020.js';

import schema from '../../../../docs/contracts/event-statistics.schema.json';
import type { EventStatisticsDto } from './event-statistics';

const ajv = new Ajv2020({ allErrors: true, strict: true });
const validate = ajv.compile<EventStatisticsDto>(schema);

export function decodeEventStatistics(value: unknown): EventStatisticsDto {
  if (!validate(value)) {
    throw new Error('Las estadísticas CHEF no cumplen su contrato.');
  }
  return value;
}
