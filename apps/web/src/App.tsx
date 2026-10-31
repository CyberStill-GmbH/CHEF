import { lazy, Suspense, useCallback, useEffect, useState } from 'react';

import type { ApiClient } from './api/ApiClient';
import { createDefaultApiClient } from './api/defaultApiClient';
import { AssetList } from './components/AssetList';
import { EmptyState } from './components/EmptyState';
import { ErrorState } from './components/ErrorState';
import { EvidencePanel } from './components/EvidencePanel';
import { LoadingState } from './components/LoadingState';
import { ReconGraphPanel } from './components/ReconGraphPanel';
import { SnapshotExportButton } from './components/SnapshotExportButton';
import type { ChefConfigDto } from './contracts/chef-config';
import type { ScanLogDto } from './contracts/scan-log';
import type { BbotEventStatisticsDto } from './contracts/event-statistics';
import { EventStatisticsPanel } from './components/EventStatisticsPanel';
import { EvidenceImportPanel } from './components/EvidenceImportPanel';
import { TargetSeedPanel } from './components/TargetSeedPanel';
import { CensysIpImportPanel } from './components/CensysIpImportPanel';
import type { CensysIpImportSummaryDto } from './contracts/censys-ip-import';
import type { ReconGraphDto } from './contracts/recon-graph';

const ScanLogTerminal = lazy(async () => {
  const module = await import('./components/ScanLogTerminal');
  return { default: module.ScanLogTerminal };
});
import { useSnapshot } from './hooks/useSnapshot';

const defaultClient = createDefaultApiClient();

interface AppProps {
  client?: ApiClient;
}

export function App({ client = defaultClient }: AppProps) {
  const [chefConfig, setChefConfig] = useState<ChefConfigDto | null>(null);
  const [configLoadFailed, setConfigLoadFailed] = useState(false);
  const [scanLogs, setScanLogs] = useState<ScanLogDto[]>([]);
  const [eventStatistics, setEventStatistics] =
    useState<BbotEventStatisticsDto | null>(null);
  const [censysImport, setCensysImport] = useState<{
    summary: CensysIpImportSummaryDto;
    graph: ReconGraphDto;
  } | null>(null);
  const onScanLog = useCallback((event: ScanLogDto) => {
    setScanLogs((current) => [...current.slice(-499), event]);
  }, []);
  const { state, retry, showSnapshot } = useSnapshot(client, onScanLog);
  const [selectedAssetId, setSelectedAssetId] = useState<string | null>(null);

  useEffect(() => {
    setEventStatistics(null);
    if (state.status !== 'success' || client.loadEventStatistics === undefined)
      return;
    const controller = new AbortController();
    void client
      .loadEventStatistics(state.snapshot.run.id, { signal: controller.signal })
      .then((result) => {
        if (!controller.signal.aborted) setEventStatistics(result.statistics);
      })
      .catch(() => {
        if (!controller.signal.aborted) setEventStatistics(null);
      });
    return () => controller.abort();
  }, [client, state]);

  useEffect(() => {
    if (client.loadConfig === undefined) return;
    const controller = new AbortController();
    void client
      .loadConfig({ signal: controller.signal })
      .then((config) => {
        if (!controller.signal.aborted) setChefConfig(config);
      })
      .catch(() => {
        if (!controller.signal.aborted) setConfigLoadFailed(true);
      });
    return () => controller.abort();
  }, [client]);

  useEffect(() => {
    if (state.status === 'empty') {
      setSelectedAssetId(null);
      return;
    }

    if (
      state.status === 'success' &&
      selectedAssetId !== null &&
      !state.snapshot.assets.some(({ id }) => id === selectedAssetId)
    ) {
      setSelectedAssetId(null);
    }
  }, [selectedAssetId, state]);

  const selectedAsset =
    state.status === 'success'
      ? state.snapshot.assets.find(({ id }) => id === selectedAssetId)
      : undefined;

  const handleBbotImported = useCallback(
    async (snapshot: import('./contracts/snapshot').SnapshotDto) => {
      let reconGraph;
      try {
        reconGraph = await client.loadReconGraph?.(snapshot.run.id);
      } catch {
        reconGraph = undefined;
      }
      showSnapshot(snapshot, reconGraph);
    },
    [client, showSnapshot],
  );
  const handleCensysImported = useCallback(
    (summary: CensysIpImportSummaryDto, graph: ReconGraphDto) => {
      setCensysImport({ summary, graph });
    },
    [],
  );

  return (
    <main>
      <h1>CHEF Frontend Smoke</h1>
      <section aria-labelledby="chef-config-title">
        <h2 id="chef-config-title">Configuración local</h2>
        {configLoadFailed && <p>No se pudo cargar config.toml de CHEF.</p>}
        {chefConfig !== null && (
          <dl>
            <dt>Scope configurado</dt>
            <dd>
              {chefConfig.config.scope.allowed_hosts.length} objetivos
              permitidos
            </dd>
            <dt>Descubrimiento de red</dt>
            <dd>
              {chefConfig.config.runtime.active_discovery
                ? 'Habilitado'
                : 'Desactivado'}
            </dd>
            <dt>Fuentes habilitadas en la configuración</dt>
            <dd>
              {
                Object.values(chefConfig.config.sources).filter(
                  ({ enabled }) => enabled,
                ).length
              }
            </dd>
          </dl>
        )}
      </section>
      {client.parseTargetSeed !== undefined && (
        <TargetSeedPanel client={client} />
      )}
      <Suspense
        fallback={
          <section aria-label="Terminal de actividad">
            Cargando terminal…
          </section>
        }
      >
        <ScanLogTerminal logs={scanLogs} />
      </Suspense>
      {client.importCensysIpResponse !== undefined &&
        client.loadCensysIpReconGraph !== undefined && (
          <CensysIpImportPanel
            client={client}
            onImported={handleCensysImported}
          />
        )}
      {censysImport !== null && (
        <section aria-labelledby="censys-import-result-title">
          <h2 id="censys-import-result-title">Evidencia Censys guardada</h2>
          <dl>
            <dt>IP consultada en el archivo</dt>
            <dd>{censysImport.summary.targetIp}</dd>
            <dt>SHA-256 de la respuesta</dt>
            <dd>{censysImport.summary.responseSha256}</dd>
            <dt>Eventos con linaje</dt>
            <dd>{censysImport.summary.eventCount}</dd>
            <dt>Activos / relaciones</dt>
            <dd>
              {censysImport.summary.assetCount} /{' '}
              {censysImport.summary.relationshipCount}
            </dd>
            <dt>Tipos aún sin proyección</dt>
            <dd>
              {Object.keys(censysImport.summary.unprojectedEventTypes)
                .length === 0
                ? 'Ninguno'
                : Object.entries(censysImport.summary.unprojectedEventTypes)
                    .map(([eventType, count]) => `${eventType}: ${count}`)
                    .join(', ')}
            </dd>
          </dl>
          <p>
            Las relaciones y activos de esta importación son pistas de un
            snapshot histórico; no representan estado actual.
          </p>
          <button type="button" onClick={() => setCensysImport(null)}>
            Volver al snapshot local
          </button>
          <ReconGraphPanel graph={censysImport.graph} />
        </section>
      )}
      {censysImport === null && state.status === 'loading' && <LoadingState />}
      {censysImport === null && state.status === 'empty' && <EmptyState />}
      {censysImport === null && state.status === 'error' && (
        <ErrorState message={state.message} onRetry={retry} />
      )}
      {state.status === 'success' && censysImport === null && (
        <>
          {client.importBbotJsonl !== undefined && (
            <EvidenceImportPanel
              client={client}
              onImported={handleBbotImported}
            />
          )}
          <section aria-labelledby="snapshot-summary-title">
            <h2 id="snapshot-summary-title">Resumen del snapshot</h2>
            <dl>
              <dt>Versión</dt>
              <dd>{state.snapshot.schemaVersion}</dd>
              <dt>Assets</dt>
              <dd>{state.snapshot.assets.length}</dd>
              <dt>Observations</dt>
              <dd>{state.snapshot.observations.length}</dd>
              <dt>Evidence</dt>
              <dd>{state.snapshot.evidence.length}</dd>
            </dl>
          </section>
          {client.exportSnapshot !== undefined && (
            <SnapshotExportButton
              client={client}
              snapshot={state.snapshot}
              hasReconGraph={state.reconGraph !== undefined}
            />
          )}
          <AssetList
            assets={state.snapshot.assets}
            selectedAssetId={selectedAssetId}
            onSelect={setSelectedAssetId}
          />
          {eventStatistics !== null && (
            <EventStatisticsPanel statistics={eventStatistics} />
          )}
          {state.reconGraph !== undefined && (
            <ReconGraphPanel graph={state.reconGraph} />
          )}
          {selectedAsset !== undefined && (
            <EvidencePanel snapshot={state.snapshot} asset={selectedAsset} />
          )}
        </>
      )}
      <footer className="legal-notice" aria-label="Licencia y atribución">
        <details>
          <summary>Licencia y avisos</summary>
          <p>
            CHEF se distribuye bajo AGPL-3.0-only. Copyright © Black Lantern
            Security y contribuidores de CHEF. El programa se ofrece sin
            garantía; puedes redistribuirlo conforme a los términos de la
            licencia.
          </p>
          <p>
            CHEF adapta capacidades de BBOT 3.0.2. Consulta los{' '}
            <a href="/LICENSE" target="_blank" rel="noreferrer">
              términos íntegros de AGPL-3.0
            </a>{' '}
            y los <a href="/THIRD_PARTY_NOTICES.md">avisos de terceros</a>.
          </p>
        </details>
      </footer>
    </main>
  );
}
