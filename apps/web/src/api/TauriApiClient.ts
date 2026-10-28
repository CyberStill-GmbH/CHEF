import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { decodeReconGraph } from '../contracts/decodeReconGraph';
import { decodeSnapshot } from '../contracts/decodeSnapshot';
import { decodeScanLog } from '../contracts/decodeScanLog';
import { decodeChefConfig } from '../contracts/decodeChefConfig';
import { decodeEventStatistics } from '../contracts/decodeEventStatistics';
import { decodeTargetSeed } from '../contracts/decodeTargetSeed';
import type { ScopePolicyDto } from '../contracts/snapshot';
import type { ApiClient } from './ApiClient';

export class TauriApiClient implements ApiClient {
  async parseTargetSeed(input: string, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const seed = await invoke<unknown>('parse_chef_target_seed', { input });
    options?.signal?.throwIfAborted();
    return decodeTargetSeed(seed);
  }

  async importBbotJsonl(
    jsonl: string,
    scope: ScopePolicyDto,
    options?: {
      signal?: AbortSignal;
      presetId?: string;
      dnsWordlist?: string;
    },
  ) {
    options?.signal?.throwIfAborted();
    const snapshot = await invoke<unknown>('import_bbot_jsonl', {
      jsonl,
      scope,
      presetId: options?.presetId ?? null,
      dnsWordlist: options?.dnsWordlist ?? null,
    });
    options?.signal?.throwIfAborted();
    return decodeSnapshot(snapshot);
  }

  async loadConfig(options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const config = await invoke<unknown>('load_chef_config');
    options?.signal?.throwIfAborted();
    return decodeChefConfig(config);
  }

  async listenScanLogs(
    listener: Parameters<NonNullable<ApiClient['listenScanLogs']>>[0],
    options?: { signal?: AbortSignal },
  ) {
    options?.signal?.throwIfAborted();
    const unlisten = await listen<unknown>('scan-log', ({ payload }) => {
      try {
        listener(decodeScanLog(payload));
      } catch {
        // Ignorar mensajes IPC que no cumplan el contrato CHEF.
      }
    });
    if (options?.signal?.aborted) {
      unlisten();
      options.signal.throwIfAborted();
    }
    return unlisten;
  }

  async loadSnapshot(options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const snapshot = await invoke<unknown>('load_offline_demo_snapshot');
    options?.signal?.throwIfAborted();
    return decodeSnapshot(snapshot);
  }

  async exportSnapshot(runId: string, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const snapshot = await invoke<unknown>('export_chef_snapshot', { runId });
    options?.signal?.throwIfAborted();
    return decodeSnapshot(snapshot);
  }

  async loadReconGraph(runId: string, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const graph = await invoke<unknown>('load_chef_recon_graph', { runId });
    options?.signal?.throwIfAborted();
    return decodeReconGraph(graph);
  }

  async loadEventStatistics(runId: string, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const statistics = await invoke<unknown>('load_chef_event_statistics', {
      runId,
    });
    options?.signal?.throwIfAborted();
    return decodeEventStatistics(statistics);
  }
}
