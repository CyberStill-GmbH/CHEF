export interface NmapEvidenceExportDto {
  schemaVersion: '1.0.0';
  runId: string;
  fileName: string;
  contentType: 'application/xml; charset=utf-8';
  scopeStatus: 'not_asserted';
  observationStart: string;
  observationEnd: string;
  hostCount: number;
  portCount: number;
  xml: string;
}
