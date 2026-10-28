export interface ObservedDomainEvidenceDto {
  id: string;
  sourceEventId: string;
  eventType: 'DNS_NAME';
  observedAt: string;
}

export interface ObservedDomainDto {
  assetId: string;
  name: string;
  qualification: 'observed';
  evidence: ObservedDomainEvidenceDto[];
}

export interface ObservedDomainExportDto {
  schemaVersion: '1.0.0';
  runId: string;
  scopeStatus: 'not_asserted';
  domains: ObservedDomainDto[];
}
