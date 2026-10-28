export interface WebEvidenceReportExportDto {
  schemaVersion: '1.0.0';
  runId: string;
  fileName: string;
  contentType: 'text/html; charset=utf-8';
  scopeStatus: 'not_asserted';
  endpointCount: number;
  candidateFindingCount: number;
  html: string;
}
