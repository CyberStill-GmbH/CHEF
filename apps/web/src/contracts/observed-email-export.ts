export interface ObservedEmailEvidenceDto {
  id: string;
  parentEvidenceId: string | null;
  observedAt: string;
}

export interface ObservedEmailDto {
  address: string;
  evidence: ObservedEmailEvidenceDto[];
}

export interface ObservedEmailExportDto {
  schemaVersion: '1.0.0';
  runId: string;
  fileName: string;
  contentType: 'application/json; charset=utf-8';
  scopeStatus: 'not_asserted';
  emailCount: number;
  emails: ObservedEmailDto[];
}
