export interface DomainAffinityEvidenceDto {
  eventId: string;
  eventType: 'DNS_NAME';
  locator: string;
  observedAt: string;
  scopeDistance: number;
  affiliateTagged: boolean;
}

export interface DomainAffinityCandidateDto {
  id: string;
  candidateDomain: string;
  analysisRoots: string[];
  reason: string;
  ruleId: string;
  ruleVersion: string;
  score: number;
  evidence: DomainAffinityEvidenceDto[];
}

export interface DomainAffinityReportDto {
  schemaVersion: '1.0.0';
  runId: string;
  analysisRoots: string[];
  candidates: DomainAffinityCandidateDto[];
}

export interface DomainAffinityRequestDto {
  schemaVersion: '1.0.0';
  runId: string;
  analysisRoots: string[];
}
