import fixture from '../../../../docs/contracts/examples/snapshot.json';
import reconGraph from '../../../../docs/contracts/examples/recon-graph.json';
import config from '../../../../docs/contracts/examples/chef-config.json';
import eventStatistics from '../../../../docs/contracts/examples/event-statistics.json';
import webEvidenceReport from '../../../../docs/contracts/examples/web-evidence-report-export.json';
import { decodeSnapshot } from '../contracts/decodeSnapshot';
import { decodeReconGraph } from '../contracts/decodeReconGraph';
import { decodeScanLog } from '../contracts/decodeScanLog';
import { decodeChefConfig } from '../contracts/decodeChefConfig';
import { decodeEventStatistics } from '../contracts/decodeEventStatistics';
import { decodeAsnReport } from '../contracts/decodeAsnReport';
import { decodeAsnReportRequest } from '../contracts/decodeAsnReportRequest';
import { decodeWebEvidenceReportExport } from '../contracts/decodeWebEvidenceReportExport';
import type { ApiClient } from './ApiClient';

export class FixtureApiClient implements ApiClient {
  async loadConfig(options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    return decodeChefConfig(config);
  }

  async listenScanLogs(
    listener: Parameters<NonNullable<ApiClient['listenScanLogs']>>[0],
    options?: { signal?: AbortSignal },
  ) {
    options?.signal?.throwIfAborted();
    const messages = [
      {
        schemaVersion: '1.0.0',
        jobId: 'job:browser-demo',
        sequence: 1,
        phase: 'started',
        message: 'Simulación offline CHEF iniciada.',
      },
      {
        schemaVersion: '1.0.0',
        jobId: 'job:browser-demo',
        sequence: 2,
        phase: 'event',
        message: 'Evento de evidencia BBOT procesado.',
        eventType: 'OPEN_TCP_PORT',
        observedAt: '2026-10-18T12:00:00Z',
      },
      {
        schemaVersion: '1.0.0',
        jobId: 'job:browser-demo',
        sequence: 3,
        phase: 'validated',
        message: 'Fixture validado; resultado listo.',
      },
      {
        schemaVersion: '1.0.0',
        jobId: 'job:browser-demo',
        sequence: 4,
        phase: 'completed',
        message: 'Simulación CHEF finalizada.',
      },
    ];
    const timers = messages.map((message, index) =>
      window.setTimeout(
        () => {
          if (!options?.signal?.aborted) listener(decodeScanLog(message));
        },
        150 + index * 500,
      ),
    );
    const stop = () => timers.forEach(window.clearTimeout);
    options?.signal?.addEventListener('abort', stop, { once: true });
    return stop;
  }

  async loadSnapshot(options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    return decodeSnapshot(fixture);
  }

  async exportSnapshot(runId: string, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const snapshot = decodeSnapshot(fixture);
    if (snapshot.run.id !== runId) throw new Error('No existe ese snapshot.');
    return snapshot;
  }

  async exportWebEvidenceReport(
    runId: string,
    options?: { signal?: AbortSignal },
  ) {
    options?.signal?.throwIfAborted();
    const exported = decodeWebEvidenceReportExport(webEvidenceReport);
    if (exported.runId !== runId)
      throw new Error('No existe ese informe CHEF.');
    return exported;
  }

  async loadReconGraph(_runId: string, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    return decodeReconGraph(reconGraph);
  }

  async loadEventStatistics(
    _runId: string,
    options?: { signal?: AbortSignal },
  ) {
    options?.signal?.throwIfAborted();
    return decodeEventStatistics(eventStatistics);
  }

  async loadAsnReport(runId: string, options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    const request = decodeAsnReportRequest({ schemaVersion: '1.0.0', runId });
    return decodeAsnReport({
      schemaVersion: '1.0.0',
      runId: request.runId,
      rows: [],
    });
  }
}
