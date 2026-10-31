import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { decodeReconGraph } from '../contracts/decodeReconGraph';
import { decodeSnapshot } from '../contracts/decodeSnapshot';
import { decodeScanLog } from '../contracts/decodeScanLog';
import { decodeChefConfig } from '../contracts/decodeChefConfig';
import { decodeEventStatistics } from '../contracts/decodeEventStatistics';
import { decodeTargetSeed } from '../contracts/decodeTargetSeed';
import { decodeObservedDomainExport } from '../contracts/decodeObservedDomainExport';
import { decodeEvidenceCsvExport } from '../contracts/decodeEvidenceCsvExport';
import { decodeNmapEvidenceExport } from '../contracts/decodeNmapEvidenceExport';
import { decodeAssetInventoryExport } from '../contracts/decodeAssetInventoryExport';
import { decodeWebEvidenceReportExport } from '../contracts/decodeWebEvidenceReportExport';
import { decodeObservedEmailExport } from '../contracts/decodeObservedEmailExport';
import { decodeCensysIpImport } from '../contracts/decodeCensysIpImport';
import type { ScanLogDto } from '../contracts/scan-log';
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
      onPhase?: (phase: ScanLogDto['phase']) => void;
    },
  ) {
    options?.signal?.throwIfAborted();
    const requestId = crypto.randomUUID();
    let jobId: string | undefined;
    let cancelable = false;
    let cancelRequest: Promise<boolean> | undefined;
    const requestCancellation = () => {
      if (!jobId || !cancelable || cancelRequest) return;
      cancelable = false;
      cancelRequest = invoke<boolean>('cancel_import_job', { jobId }).catch(
        () => false,
      );
    };
    const unlisten = await listen<unknown>('scan-log', ({ payload }) => {
      try {
        const event = decodeScanLog(payload);
        if (event.requestId !== requestId) return;
        options?.onPhase?.(event.phase);
        if (event.phase === 'started') {
          jobId = event.jobId;
          cancelable = true;
          if (options?.signal?.aborted) requestCancellation();
        } else if (
          event.phase === 'validated' ||
          event.phase === 'stored' ||
          event.phase === 'completed' ||
          event.phase === 'failed' ||
          event.phase === 'cancelled'
        ) {
          cancelable = false;
        }
      } catch {
        // Ignorar mensajes IPC que no cumplan el contrato CHEF.
      }
    });
    const onAbort = () => requestCancellation();
    options?.signal?.addEventListener('abort', onAbort, { once: true });
    try {
      if (options?.signal?.aborted) requestCancellation();
      const snapshot = await invoke<unknown>('import_bbot_jsonl', {
        requestId,
        jsonl,
        scope,
        presetId: options?.presetId ?? null,
      });
      if (cancelRequest && (await cancelRequest)) throw abortError();
      return decodeSnapshot(snapshot);
    } catch (error) {
      if (cancelRequest && (await cancelRequest)) throw abortError();
      throw error;
    } finally {
      options?.signal?.removeEventListener('abort', onAbort);
      unlisten();
    }
  }

  async importCensysIpResponse(
    targetIp: string,
    responseJson: string,
    options?: { onPhase?: (phase: ScanLogDto['phase']) => void },
  ) {
    const requestId = crypto.randomUUID();
    const unlisten = await listen<unknown>('scan-log', ({ payload }) => {
      try {
        const event = decodeScanLog(payload);
        if (event.requestId === requestId) options?.onPhase?.(event.phase);
      } catch {
        // Ignorar mensajes IPC que no cumplan el contrato CHEF.
      }
    });
    try {
      const summary = decodeCensysIpImport(
        await invoke<unknown>('import_censys_host_response', {
          requestId,
          targetIp,
          responseJson,
        }),
      );
      return summary;
    } finally {
      unlisten();
    }
  }

  async loadCensysIpReconGraph(
    importId: string,
    options?: { signal?: AbortSignal },
  ) {
    options?.signal?.throwIfAborted();
    const graph = await invoke<unknown>('load_censys_ip_recon_graph', {
      importId,
    });
    options?.signal?.throwIfAborted();
    return decodeReconGraph(graph);
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

  async exportObservedDomains(
    runId: string,
    options?: { signal?: AbortSignal },
  ) {
    options?.signal?.throwIfAborted();
    const exported = await invoke<unknown>('export_chef_observed_domains', {
      runId,
    });
    options?.signal?.throwIfAborted();
    return decodeObservedDomainExport(exported);
  }

  async exportEvidenceCsv(runId: string, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const exported = await invoke<unknown>('export_chef_evidence_csv', {
      runId,
    });
    options?.signal?.throwIfAborted();
    return decodeEvidenceCsvExport(exported);
  }

  async exportNmapEvidence(runId: string, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const exported = await invoke<unknown>('export_chef_nmap_evidence', {
      runId,
    });
    options?.signal?.throwIfAborted();
    return decodeNmapEvidenceExport(exported);
  }

  async exportAssetInventory(
    runId: string,
    options?: { signal?: AbortSignal },
  ) {
    options?.signal?.throwIfAborted();
    const exported = await invoke<unknown>('export_chef_asset_inventory', {
      runId,
    });
    options?.signal?.throwIfAborted();
    return decodeAssetInventoryExport(exported);
  }

  async exportWebEvidenceReport(
    runId: string,
    options?: { signal?: AbortSignal },
  ) {
    options?.signal?.throwIfAborted();
    const exported = await invoke<unknown>('export_chef_web_evidence_report', {
      runId,
    });
    options?.signal?.throwIfAborted();
    return decodeWebEvidenceReportExport(exported);
  }

  async exportObservedEmails(
    runId: string,
    options?: { signal?: AbortSignal },
  ) {
    options?.signal?.throwIfAborted();
    const exported = await invoke<unknown>('export_chef_observed_emails', {
      runId,
    });
    options?.signal?.throwIfAborted();
    return decodeObservedEmailExport(exported);
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

function abortError() {
  return new DOMException('Importación cancelada.', 'AbortError');
}
