export interface AsnReportEvidenceDto {
  ip: string;
  ipEventId: string;
  ipLocator: string;
  ipObservedAt: string;
  asnEventId: string;
  asnLocator: string;
  asnObservedAt: string;
  asnModule: string | null;
}

export interface AsnReportRowDto {
  asn: number;
  ips: string[];
  names: string[];
  descriptions: string[];
  countries: string[];
  preservedSubnetCounts: number[];
  evidence: AsnReportEvidenceDto[];
}

export interface AsnReportRequestDto {
  schemaVersion: '1.0.0';
  runId: string;
}

export interface AsnReportDto {
  schemaVersion: '1.0.0';
  runId: string;
  rows: AsnReportRowDto[];
}
