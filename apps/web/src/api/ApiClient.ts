import type { SnapshotDto } from '../contracts/snapshot';
import type { ReconGraphDto } from '../contracts/recon-graph';
import type { ScanLogDto, ScanLogPhase } from '../contracts/scan-log';
import type { ChefConfigDto } from '../contracts/chef-config';
import type { EventStatisticsDto } from '../contracts/event-statistics';
import type { ScopePolicyDto } from '../contracts/snapshot';
import type { TargetSeedDto } from '../contracts/target-seed';
import type { ObservedDomainExportDto } from '../contracts/observed-domain-export';
import type { EvidenceCsvExportDto } from '../contracts/evidence-csv-export';
import type { NmapEvidenceExportDto } from '../contracts/nmap-evidence-export';
import type { AssetInventoryExportDto } from '../contracts/asset-inventory-export';
import type { WebEvidenceReportExportDto } from '../contracts/web-evidence-report-export';

export interface ApiClient {
  loadSnapshot(options?: { signal?: AbortSignal }): Promise<SnapshotDto>;
  exportSnapshot?(
    runId: string,
    options?: { signal?: AbortSignal },
  ): Promise<SnapshotDto>;
  exportObservedDomains?(
    runId: string,
    options?: { signal?: AbortSignal },
  ): Promise<ObservedDomainExportDto>;
  exportEvidenceCsv?(
    runId: string,
    options?: { signal?: AbortSignal },
  ): Promise<EvidenceCsvExportDto>;
  exportNmapEvidence?(
    runId: string,
    options?: { signal?: AbortSignal },
  ): Promise<NmapEvidenceExportDto>;
  exportAssetInventory?(
    runId: string,
    options?: { signal?: AbortSignal },
  ): Promise<AssetInventoryExportDto>;
  exportWebEvidenceReport?(
    runId: string,
    options?: { signal?: AbortSignal },
  ): Promise<WebEvidenceReportExportDto>;
  loadReconGraph?(
    runId: string,
    options?: { signal?: AbortSignal },
  ): Promise<ReconGraphDto>;
  loadEventStatistics?(
    runId: string,
    options?: { signal?: AbortSignal },
  ): Promise<EventStatisticsDto>;
  importBbotJsonl?(
    jsonl: string,
    scope: ScopePolicyDto,
    options?: {
      signal?: AbortSignal;
      presetId?: string;
      onPhase?: (phase: ScanLogPhase) => void;
    },
  ): Promise<SnapshotDto>;
  listenScanLogs?(
    listener: (event: ScanLogDto) => void,
    options?: { signal?: AbortSignal },
  ): Promise<() => void>;
  loadConfig?(options?: { signal?: AbortSignal }): Promise<ChefConfigDto>;
  parseTargetSeed?(
    input: string,
    options?: { signal?: AbortSignal },
  ): Promise<TargetSeedDto>;
}
