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
import { UrlScanImportPanel } from './components/UrlScanImportPanel';
import type { UrlScanImportSummaryDto } from './contracts/urlscan-import';

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
  const [urlscanImport, setUrlscanImport] = useState<{
    summary: UrlScanImportSummaryDto;
    graph: ReconGraphDto;
  } | null>(null);
  const onScanLog = useCallback((event: ScanLogDto) => {
    setScanLogs((current) => [...current.slice(-499), event]);
  }, []);
  const { state, retry, showSnapshot } = useSnapshot(client, onScanLog);
  const [selectedAssetId, setSelectedAssetId] = useState<string | null>(null);
  const [activeWorkspace, setActiveWorkspace] = useState('recon');

  const runChefCommand = useCallback(
    async (rawCommand: string): Promise<string[]> => {
      const command = rawCommand.trim();
      if (command === 'chef help') {
        return [
          'Comandos: chef status · chef url <URL> · chef report · clear',
          'La terminal CHEF no ejecuta comandos del sistema ni inicia escaneos.',
        ];
      }

      if (command === 'chef status') {
        if (state.status === 'loading') return ['Cargando snapshot…'];
        if (state.status === 'error')
          return ['Snapshot no disponible. Usa Reintentar.'];
        if (state.status === 'empty') return ['Snapshot cargado: sin activos.'];
        return [
          `Snapshot listo · ${state.snapshot.assets.length} activos · ${state.snapshot.evidence.length} evidencias`,
        ];
      }

      if (command.startsWith('chef url ')) {
        const rawUrl = command.slice('chef url '.length).trim();
        if (client.parseTargetSeed === undefined)
          return ['El analizador de URL no está disponible en este entorno.'];
        let target: URL;
        try {
          target = new URL(rawUrl);
        } catch {
          return ['URL inválida. Usa https://dominio/ruta.'];
        }
        if (
          !['http:', 'https:'].includes(target.protocol) ||
          target.username.length > 0 ||
          target.password.length > 0
        ) {
          return ['Usa una URL HTTP o HTTPS sin credenciales.'];
        }
        target.search = '';
        target.hash = '';
        const safeUrl = target.toString();
        try {
          const parsed = await client.parseTargetSeed(safeUrl);
          if (parsed.kind !== 'url') return ['No se reconoció una URL válida.'];
          return [
            `URL preparada: ${parsed.data}`,
            'Análisis local completado. No se hizo ninguna consulta de red.',
          ];
        } catch {
          return ['No se pudo clasificar la URL. Revisa el formato e inténtalo otra vez.'];
        }
      }

      if (command === 'chef report') {
        if (state.status !== 'success')
          return ['Carga un snapshot antes de generar el informe.'];
        if (client.exportWebEvidenceReport === undefined)
          return ['La exportación de informes no está disponible en este entorno.'];
        try {
          const report = await client.exportWebEvidenceReport(state.snapshot.run.id);
          const objectUrl = URL.createObjectURL(
            new Blob([report.html], { type: report.contentType }),
          );
          const anchor = document.createElement('a');
          anchor.href = objectUrl;
          anchor.download = safeDownloadName(report.fileName);
          anchor.hidden = true;
          document.body.append(anchor);
          anchor.click();
          anchor.remove();
          window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
          return [`Informe descargado: ${anchor.download}`];
        } catch {
          return ['No se pudo generar el informe. Inténtalo de nuevo.'];
        }
      }

      return ['Comando no reconocido. Escribe chef help.'];
    },
    [client, state],
  );

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
      setCensysImport(null);
      setUrlscanImport(null);
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
      setUrlscanImport(null);
    },
    [],
  );
  const handleUrlScanImported = useCallback(
    (summary: UrlScanImportSummaryDto, graph: ReconGraphDto) => {
      setUrlscanImport({ summary, graph });
      setCensysImport(null);
    },
    [],
  );

  function navigateTo(section: string, targetId: string) {
    setActiveWorkspace(section);
    if (section === 'evidence') {
      setCensysImport(null);
      setUrlscanImport(null);
    }
    requestAnimationFrame(() =>
      (
        document.getElementById(targetId) ?? document.getElementById('overview')
      )?.scrollIntoView({ behavior: 'smooth', block: 'start' }),
    );
  }

  return (
    <div className="chef-shell">
      <aside className="project-rail" aria-label="Navegación principal">
        <a className="brand-mark" href="#workspace" aria-label="CHEF, inicio">
          <img src="/chef-mark.svg" alt="" />
        </a>
        <button
          className={
            activeWorkspace === 'recon'
              ? 'rail-action is-active'
              : 'rail-action'
          }
          type="button"
          aria-label="Reconocimiento"
          aria-current={activeWorkspace === 'recon' ? 'page' : undefined}
          onClick={() => navigateTo('recon', 'overview')}
        >
          <span aria-hidden="true">⌕</span>
        </button>
        <button
          className={
            activeWorkspace === 'evidence'
              ? 'rail-action is-active'
              : 'rail-action'
          }
          type="button"
          aria-label="Evidencias"
          aria-current={activeWorkspace === 'evidence' ? 'page' : undefined}
          onClick={() => navigateTo('evidence', 'evidence-workspace')}
        >
          <span aria-hidden="true">◈</span>
        </button>
        <button
          className={
            activeWorkspace === 'sources'
              ? 'rail-action is-active'
              : 'rail-action'
          }
          type="button"
          aria-label="Fuentes archivadas"
          aria-current={activeWorkspace === 'sources' ? 'page' : undefined}
          onClick={() => navigateTo('sources', 'source-imports')}
        >
          <span aria-hidden="true">⇣</span>
        </button>
        <div className="rail-spacer" />
        <button
          className={
            activeWorkspace === 'settings'
              ? 'rail-action is-active'
              : 'rail-action'
          }
          type="button"
          aria-label="Configuración"
          aria-current={activeWorkspace === 'settings' ? 'page' : undefined}
          onClick={() => navigateTo('settings', 'chef-config-title')}
        >
          <span aria-hidden="true">⚙</span>
        </button>
        <span className="rail-version">0.1</span>
      </aside>
      <aside className="project-sidebar" aria-label="Proyecto local">
        <a className="wordmark" href="#workspace">
          <img src="/chef-wordmark.svg" alt="CHEF" />
        </a>
        <div className="project-switcher">
          <span className="project-avatar" aria-hidden="true">
            <img src="/chef-mascot.webp" alt="" width="30" height="30" />
          </span>
          <span>
            <strong>Proyecto local</strong>
            <small>Espacio de trabajo</small>
          </span>
          <span className="switcher-chevron" aria-hidden="true">
            ⌄
          </span>
        </div>
        <p className="sidebar-label">ESPACIO DE TRABAJO</p>
        <nav className="workspace-nav" aria-label="Vistas del proyecto">
          <button
            className={
              activeWorkspace === 'recon' ? 'nav-item is-active' : 'nav-item'
            }
            type="button"
            onClick={() => navigateTo('recon', 'overview')}
          >
            <span aria-hidden="true">⌕</span> Reconocimiento
          </button>
          <button
            className={
              activeWorkspace === 'evidence' ? 'nav-item is-active' : 'nav-item'
            }
            type="button"
            onClick={() => navigateTo('evidence', 'evidence-workspace')}
          >
            <span aria-hidden="true">◈</span> Evidencia
          </button>
          <button
            className={
              activeWorkspace === 'sources' ? 'nav-item is-active' : 'nav-item'
            }
            type="button"
            onClick={() => navigateTo('sources', 'source-imports')}
          >
            <span aria-hidden="true">⇣</span> Importaciones
          </button>
        </nav>
        <div className="sidebar-divider" />
        <p className="sidebar-label">ESTADO LOCAL</p>
        <div className="sidebar-status">
          <i aria-hidden="true" /> Entorno local
        </div>
        <p className="sidebar-note">
          Trabajo offline. Las fuentes archivadas se procesan en este equipo.
        </p>
        <div className="sidebar-bottom">
          <span className="avatar-dot">CS</span>
          <span>
            <strong>CHEF Desktop</strong>
            <small>Sesión local</small>
          </span>
        </div>
      </aside>
      <main id="workspace" className="chef-main">
        <header className="topbar">
          <div className="breadcrumbs">
            <span>CHEF</span>
            <b>/</b>
            <strong>Proyecto local</strong>
          </div>
          <div className="topbar-actions">
            <span className="offline-pill">
              <i aria-hidden="true" /> Sin conexión
            </span>
          </div>
        </header>
        <div className="main-content">
          <section className="page-heading" id="overview">
            <div>
              <h1>Mapa de activos</h1>
              <p className="page-description">
                Activos y evidencia local.
              </p>
            </div>
            <div className="workspace-badge">
              <span className="status-dot" />{' '}
              {state.status === 'success'
                ? `${state.snapshot.assets.length} activos`
                : 'Sin snapshot'}
            </div>
          </section>
          <section
            aria-labelledby="chef-config-title"
            id="settings-panel"
            className="config-strip"
          >
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
          <div id="source-imports" className="imports-workspace">
            {client.importCensysIpResponse !== undefined &&
              client.loadCensysIpReconGraph !== undefined && (
                <CensysIpImportPanel
                  client={client}
                  onImported={handleCensysImported}
                />
              )}
            {censysImport !== null && (
              <section aria-labelledby="censys-import-result-title">
                <h2 id="censys-import-result-title">
                  Evidencia Censys guardada
                </h2>
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
                      : Object.entries(
                          censysImport.summary.unprojectedEventTypes,
                        )
                          .map(([eventType, count]) => `${eventType}: ${count}`)
                          .join(', ')}
                  </dd>
                </dl>
                <p>
                  Las relaciones y activos de esta importación son pistas de un
                  snapshot histórico; no representan estado actual.
                </p>
                <button
                  type="button"
                  onClick={() => navigateTo('evidence', 'evidence-workspace')}
                >
                  Volver al snapshot local
                </button>
                <ReconGraphPanel graph={censysImport.graph} />
              </section>
            )}
            {client.importUrlScanResponse !== undefined &&
              client.loadUrlScanReconGraph !== undefined && (
                <UrlScanImportPanel
                  client={client}
                  onImported={handleUrlScanImported}
                />
              )}
            {urlscanImport !== null && (
              <section aria-labelledby="urlscan-import-result-title">
                <h2 id="urlscan-import-result-title">
                  Evidencia urlscan guardada
                </h2>
                <dl>
                  <dt>Dominio consultado</dt>
                  <dd>{urlscanImport.summary.queryDomain}</dd>
                  <dt>SHA-256 de la respuesta</dt>
                  <dd>{urlscanImport.summary.responseSha256}</dd>
                  <dt>Eventos con linaje</dt>
                  <dd>{urlscanImport.summary.eventCount}</dd>
                  <dt>Activos / relaciones</dt>
                  <dd>
                    {urlscanImport.summary.assetCount} /{' '}
                    {urlscanImport.summary.relationshipCount}
                  </dd>
                </dl>
                <p>
                  Las pistas históricas del archivo no representan estado DNS
                  actual.
                </p>
                <button
                  type="button"
                  onClick={() => navigateTo('evidence', 'evidence-workspace')}
                >
                  Volver al snapshot local
                </button>
                <ReconGraphPanel graph={urlscanImport.graph} />
              </section>
            )}
          </div>
          {censysImport === null &&
            urlscanImport === null &&
            state.status === 'loading' && <LoadingState />}
          {censysImport === null &&
            urlscanImport === null &&
            state.status === 'empty' && <EmptyState />}
          {censysImport === null &&
            urlscanImport === null &&
            state.status === 'error' && (
              <ErrorState message={state.message} onRetry={retry} />
            )}
          {state.status === 'success' &&
            censysImport === null &&
            urlscanImport === null && (
              <>
                {client.importBbotJsonl !== undefined && (
                  <EvidenceImportPanel
                    client={client}
                    onImported={handleBbotImported}
                  />
                )}
                <section
                  aria-labelledby="snapshot-summary-title"
                  id="snapshot-summary"
                  className="snapshot-summary"
                >
                  <h2 id="snapshot-summary-title">Resumen del snapshot</h2>
                  <dl>
                    <dt>Versión</dt>
                    <dd>{state.snapshot.schemaVersion}</dd>
                    <dt>Activos</dt>
                    <dd>{state.snapshot.assets.length}</dd>
                    <dt>Observaciones</dt>
                    <dd>{state.snapshot.observations.length}</dd>
                    <dt>Evidencias</dt>
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
                {eventStatistics !== null && (
                  <EventStatisticsPanel statistics={eventStatistics} />
                )}
                <div className="explorer-grid" id="evidence-workspace">
                  <section
                    className="explorer-pane asset-pane"
                    aria-labelledby="asset-list-title"
                  >
                    <header className="pane-heading">
                      <h2 id="asset-list-title">Activos</h2>
                      <span>{state.snapshot.assets.length}</span>
                    </header>
                    <AssetList
                      assets={state.snapshot.assets}
                      selectedAssetId={selectedAssetId}
                      onSelect={setSelectedAssetId}
                    />
                  </section>
                  <section
                    className="explorer-pane graph-pane"
                    aria-labelledby="graph-title"
                  >
                    <header className="pane-heading">
                      <h2 id="graph-title">Mapa de relaciones</h2>
                      <span className="pane-meta">Evidencia local</span>
                    </header>
                    {state.reconGraph !== undefined && (
                      <ReconGraphPanel graph={state.reconGraph} />
                    )}
                    {state.reconGraph === undefined && (
                      <p className="pane-empty">
                        El snapshot aún no tiene relaciones para explorar.
                      </p>
                    )}
                  </section>
                  <aside
                    className="explorer-pane inspector-pane"
                    aria-labelledby="inspector-title"
                  >
                    <header className="pane-heading">
                      <h2 id="inspector-title">Inspector</h2>
                    </header>
                    {selectedAsset !== undefined ? (
                      <EvidencePanel
                        snapshot={state.snapshot}
                        asset={selectedAsset}
                      />
                    ) : (
                      <p className="pane-empty">
                        Selecciona un activo para revisar su evidencia,
                        procedencia y relaciones.
                      </p>
                    )}
                  </aside>
                </div>
              </>
            )}
          <Suspense
            fallback={
              <section
                aria-label="Terminal de actividad"
                className="scan-log-panel"
              >
                Cargando terminal de actividad…
              </section>
            }
          >
            <div id="activity-terminal">
              <ScanLogTerminal logs={scanLogs} onCommand={runChefCommand} />
            </div>
          </Suspense>
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
                Consulta los{' '}
                <a href="/LICENSE" target="_blank" rel="noreferrer">
                  términos íntegros de AGPL-3.0
                </a>{' '}
                y los <a href="/THIRD_PARTY_NOTICES.md">avisos de terceros</a>.
              </p>
            </details>
          </footer>
        </div>
      </main>
    </div>
  );
}

function safeDownloadName(value: string) {
  const name = value.replace(/[\\/:*?"<>|\u0000-\u001f]/g, '-').trim();
  return name.length > 0 ? name : 'chef-informe.html';
}
