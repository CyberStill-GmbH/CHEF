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
import { DomainAffinityPanel } from './components/DomainAffinityPanel';

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
      {state.status === 'loading' && <LoadingState />}
      {state.status === 'empty' && <EmptyState />}
      {state.status === 'error' && (
        <ErrorState message={state.message} onRetry={retry} />
      )}
      {state.status === 'success' && (
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
          {client.analyzeDomainAffinity !== undefined &&
            state.reconGraph !== undefined && (
              <DomainAffinityPanel
                key={state.snapshot.run.id}
                client={client}
                runId={state.snapshot.run.id}
                reconGraph={state.reconGraph}
              />
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
