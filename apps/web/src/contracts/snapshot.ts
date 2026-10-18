export type SnapshotSchemaVersion = '1.0.0';
export type NetworkProtocol = 'tcp' | 'udp';

export interface ScopePolicyDto {
  schemaVersion: SnapshotSchemaVersion;
  id: string;
  allowedAddresses: string[];
  excludedAddresses: string[];
  allowedPorts: number[];
  allowedProtocols: NetworkProtocol[];
  maxInputBytes: number;
  maxObservations: number;
}

export interface ScanRunDto {
  id: string;
  scopePolicyId: string;
  scopeDigest: string;
  state: 'completed';
  startedAt: string;
  finishedAt: string;
}

export interface EvidenceDto {
  id: string;
  sha256: string;
  source: 'nmap-xml';
  parserVersion: SnapshotSchemaVersion;
  runId: string;
  locator: string;
  importedAt: string;
}

export interface ObservationDto {
  id: string;
  address: string;
  protocol: NetworkProtocol;
  port: number;
  observedAt: string;
  locator: string;
  evidenceId: string;
  runId: string;
  source: 'nmap-xml';
  fact: 'open-port';
  confidence: number;
}

export interface AssetDto {
  id: string;
  kind: 'address' | 'service';
  key: string;
  firstSeen: string;
  lastSeen: string;
  observationIds: string[];
}

export interface RelationshipDto {
  id: string;
  from: string;
  to: string;
  kind: 'exposes';
  assertion: 'observed';
  observationIds: string[];
  confidence: number;
}

export interface ExposurePathDto {
  id: string;
  assetIds: string[];
  relationshipIds: string[];
  assertion: 'inferred';
  ruleId: 'open-service/v1';
  explanation: string;
  confidence: number;
  isConfirmedVulnerability: false;
}

export interface SnapshotDto {
  schemaVersion: SnapshotSchemaVersion;
  scopePolicy: ScopePolicyDto;
  run: ScanRunDto;
  evidence: EvidenceDto[];
  observations: ObservationDto[];
  assets: AssetDto[];
  relationships: RelationshipDto[];
  paths: ExposurePathDto[];
}
