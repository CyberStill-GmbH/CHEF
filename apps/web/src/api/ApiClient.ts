import type { SnapshotDto } from '../contracts/snapshot';
import type { ReconGraphDto } from '../contracts/recon-graph';
import type { ScanLogDto } from '../contracts/scan-log';
import type { ChefConfigDto } from '../contracts/chef-config';
import type { EventStatisticsDto } from '../contracts/event-statistics';
import type { ScopePolicyDto } from '../contracts/snapshot';
import type { TargetSeedDto } from '../contracts/target-seed';

export interface ApiClient {
  loadSnapshot(options?: { signal?: AbortSignal }): Promise<SnapshotDto>;
  exportSnapshot?(
    runId: string,
    options?: { signal?: AbortSignal },
  ): Promise<SnapshotDto>;
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
      dnsWordlist?: string;
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
