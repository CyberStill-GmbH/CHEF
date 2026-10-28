import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import exportSchema from '../../../../docs/contracts/asset-inventory-export.schema.json';
import type { AssetInventoryExportDto } from './asset-inventory-export';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
const validate = ajv.compile<AssetInventoryExportDto>(exportSchema);

export function decodeAssetInventoryExport(
  value: unknown,
): AssetInventoryExportDto {
  if (!validate(value)) {
    throw new Error('La exportación de inventario CHEF no es válida.');
  }
  return value;
}
