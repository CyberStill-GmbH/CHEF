export interface CensysIpImportSummaryDto {
  schemaVersion: '1.0.0';
  importId: string;
  targetIp: string;
  responseSha256: string;
  importedAt: string;
  eventCount: number;
  assetCount: number;
  relationshipCount: number;
  unprojectedEventTypes: Record<string, number>;
}
