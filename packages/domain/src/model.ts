export const SCHEMA_VERSION = '1.0.0' as const;
export type Protocol = 'tcp' | 'udp';

export interface ScopePolicy {
  schemaVersion: typeof SCHEMA_VERSION;
  id: string;
  allowedAddresses: string[];
  excludedAddresses: string[];
  allowedPorts: number[];
  allowedProtocols: Protocol[];
  maxInputBytes: number;
  maxObservations: number;
}

export interface ServiceInput {
  address: string;
  protocol: Protocol;
  port: number;
  observedAt: string;
  locator: string;
}

export interface Asset {
  id: string;
  kind: 'address' | 'service';
  key: string;
  firstSeen: string;
  lastSeen: string;
  observationIds: string[];
}

export interface Evidence {
  id: string;
  sha256: string;
  source: 'nmap-xml';
  parserVersion: string;
  runId: string;
  locator: string;
  importedAt: string;
}

export interface Observation extends ServiceInput {
  id: string;
  evidenceId: string;
  runId: string;
  source: 'nmap-xml';
  fact: 'open-port';
  confidence: number;
}

export interface Relationship {
  id: string;
  from: string;
  to: string;
  kind: 'exposes';
  assertion: 'observed';
  observationIds: string[];
  confidence: number;
}

export interface ExposurePath {
  id: string;
  assetIds: string[];
  relationshipIds: string[];
  assertion: 'inferred';
  ruleId: 'open-service/v1';
  explanation: string;
  confidence: number;
  isConfirmedVulnerability: false;
}

export interface ScanRun {
  id: string;
  scopePolicyId: string;
  scopeDigest: string;
  state: 'completed';
  startedAt: string;
  finishedAt: string;
}

export interface Snapshot {
  schemaVersion: typeof SCHEMA_VERSION;
  scopePolicy: ScopePolicy;
  run: ScanRun;
  evidence: Evidence[];
  observations: Observation[];
  assets: Asset[];
  relationships: Relationship[];
  paths: ExposurePath[];
}
