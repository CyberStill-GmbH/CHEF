export interface AssetInventoryExportDto {
  schemaVersion: '1.0.0';
  runId: string;
  fileName: string;
  contentType: 'text/csv; charset=utf-8';
  scopeStatus: 'not_asserted';
  rowCount: number;
  csv: string;
}
