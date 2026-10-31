export interface UrlScanImportSummaryDto {
  schemaVersion: '1.0.0';
  importId: string;
  queryDomain: string;
  responseSha256: string;
  importedAt: string;
  eventCount: number;
  assetCount: number;
  relationshipCount: number;
  unprojectedEventTypes: Record<string, number>;
}
