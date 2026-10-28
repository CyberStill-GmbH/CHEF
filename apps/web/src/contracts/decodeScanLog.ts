import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import scanLogSchema from '../../../../docs/contracts/scan-log.schema.json';
import type { ScanLogDto } from './scan-log';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validateScanLog = ajv.compile<ScanLogDto>(scanLogSchema);

export function decodeScanLog(value: unknown): ScanLogDto {
  if (!validateScanLog(value))
    throw new Error('Evento de actividad CHEF no válido.');
  const event = value as ScanLogDto;
  if (
    (event.phase === 'event' &&
      (event.eventType === undefined || event.observedAt === undefined)) ||
    (event.phase !== 'event' &&
      (event.eventType !== undefined || event.observedAt !== undefined))
  ) {
    throw new Error('Los campos del evento CHEF no corresponden con su fase.');
  }
  return value;
}
