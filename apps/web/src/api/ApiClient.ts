import type { SnapshotDto } from '../contracts/snapshot';

export interface ApiClient {
  loadSnapshot(options?: { signal?: AbortSignal }): Promise<SnapshotDto>;
}
