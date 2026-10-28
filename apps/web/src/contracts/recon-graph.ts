export type ReconAssetKind =
  | 'domain'
  | 'ip_address'
  | 'service'
  | 'web_endpoint'
  | 'technology'
  | 'certificate'
  | 'candidate_finding'
  | 'organization';

export type ReconRelationshipKind =
  | 'resolves_to'
  | 'parent_domain_of'
  | 'exposes_service'
  | 'serves_endpoint'
  | 'uses_technology'
  | 'presents_certificate'
  | 'reports_candidate';

export type ReconAssertion = 'observed' | 'candidate' | 'unverified' | 'hint';

export interface ReconAssetDto {
  id: string;
  kind: ReconAssetKind;
  key: string;
  qualification: string | null;
  firstSeen: string;
  lastSeen: string;
  evidenceIds: string[];
}

export interface ReconEvidenceDto {
  id: string;
  sourceEventId: string;
  parentEvidenceId: string | null;
  eventType: string;
  observedAt: string;
  locator: string;
  module: string | null;
  redacted: boolean;
}

export interface ReconRelationshipDto {
  id: string;
  fromAssetId: string;
  toAssetId: string;
  kind: ReconRelationshipKind;
  assertion: ReconAssertion;
  evidenceIds: string[];
}

export interface ReconGraphDto {
  schemaVersion: '1.0.0' | '1.1.0' | '1.2.0' | '1.3.0' | '1.4.0';
  assets: ReconAssetDto[];
  evidence: ReconEvidenceDto[];
  relationships: ReconRelationshipDto[];
  unmappedEventTypes: Record<string, number>;
}
