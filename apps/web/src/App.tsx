import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';

import { AmbientHueContext } from './ambient';
import type { ApiClient } from './api/ApiClient';
import { createDefaultApiClient } from './api/defaultApiClient';
import {
  AURORA_PRESETS,
  AuroraBackground,
  auroraLabel,
  auroraTone,
  type AuroraPreset,
} from './components/AuroraBackground';
import { AssetList } from './components/AssetList';
import { CountUp } from './components/CountUp';
import { GlobeSpacer, GlobeStage } from './components/GlobeFooter';
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
import { CensysIpImportPanel } from './components/CensysIpImportPanel';
import type { CensysIpImportSummaryDto } from './contracts/censys-ip-import';
import type { ReconGraphDto } from './contracts/recon-graph';
import type { ReconGraphProgressDto } from './contracts/recon-graph-progress';
import { UrlScanImportPanel } from './components/UrlScanImportPanel';
import type { UrlScanImportSummaryDto } from './contracts/urlscan-import';

const ScanLogTerminal = lazy(async () => {
  const module = await import('./components/ScanLogTerminal');
  return { default: module.ScanLogTerminal };
});
import { useSnapshot } from './hooks/useSnapshot';

const defaultClient = createDefaultApiClient();

const AMBIENT_KEY = 'chef.ambient.v2';

function readAmbient(): AuroraPreset {
  try {
    const stored = window.localStorage.getItem(AMBIENT_KEY);
    if (stored !== null && (AURORA_PRESETS as string[]).includes(stored))
      return stored as AuroraPreset;
  } catch {
    // El almacenamiento puede estar bloqueado; se usa el ambiente por defecto.
  }
  return 'dune';
}

interface AppProps {
  client?: ApiClient;
}

export function App({ client = defaultClient }: AppProps) {
  const [chefConfig, setChefConfig] = useState<ChefConfigDto | null>(null);
  const [configLoadFailed, setConfigLoadFailed] = useState(false);
  const [scanLogs, setScanLogs] = useState<ScanLogDto[]>([]);
  const [streamedGraph, setStreamedGraph] = useState<ReconGraphDto | null>(
    null,
  );
  const [streamedGraphRequestId, setStreamedGraphRequestId] = useState<
    string | null
  >(null);
  const streamedGraphRef = useRef<{
    requestId: string;
    sequence: number;
    graph: ReconGraphDto;
  } | null>(null);
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
    if (event.phase === 'started') {
      streamedGraphRef.current = null;
      setStreamedGraph(null);
      setStreamedGraphRequestId(null);
    } else if (event.phase === 'failed' || event.phase === 'cancelled') {
      streamedGraphRef.current = null;
      setStreamedGraph(null);
      setStreamedGraphRequestId(null);
    }
  }, []);
  const { state, retry, showSnapshot } = useSnapshot(client, onScanLog);
  const [selectedAssetId, setSelectedAssetId] = useState<string | null>(null);
  const [activeWorkspace, setActiveWorkspace] = useState('recon');
  const [ambient, setAmbient] = useState<AuroraPreset>(readAmbient);
  const accentHue = auroraTone(ambient).hue;

  // Todo el tinte de la interfaz sale del matiz del ambiente (variable CSS).
  useEffect(() => {
    document.documentElement.style.setProperty('--chef-hue', String(accentHue));
  }, [accentHue]);
  // Avance (0..1) con el que el planeta final se asoma según el scroll.
  const planetProgress = useRef(0);
  const [consoleOpen, setConsoleOpen] = useState(true);

  useEffect(() => {
    try {
      window.localStorage.setItem(AMBIENT_KEY, ambient);
    } catch {
      // Sin almacenamiento: el ambiente simplemente no se recuerda.
    }
  }, [ambient]);

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
          return [
            'No se pudo clasificar la URL. Revisa el formato e inténtalo otra vez.',
          ];
        }
      }

      if (command === 'chef report') {
        if (state.status !== 'success')
          return ['Carga un snapshot antes de generar el informe.'];
        if (client.exportWebEvidenceReport === undefined)
          return [
            'La exportación de informes no está disponible en este entorno.',
          ];
        try {
          const report = await client.exportWebEvidenceReport(
            state.snapshot.run.id,
          );
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
  const activeRunLabel =
    state.status === 'success'
      ? `Ejecución ${state.snapshot.run.id.slice(-8)}`
      : 'Sin ejecución cargada';
  const backendStatus =
    client.runtimeMode === 'demo'
      ? 'Vista previa offline'
      : state.status === 'success' || state.status === 'empty'
        ? 'Backend CHEF conectado'
        : state.status === 'error'
          ? 'Backend no disponible'
          : 'Conectando al backend';
  const backendStatusClass =
    client.runtimeMode === 'demo'
      ? 'is-demo'
      : state.status === 'error'
        ? 'is-error'
        : state.status === 'success' || state.status === 'empty'
          ? 'is-ready'
          : 'is-pending';

  const handleBbotImported = useCallback(
    async (snapshot: import('./contracts/snapshot').SnapshotDto) => {
      streamedGraphRef.current = null;
      setStreamedGraph(null);
      setStreamedGraphRequestId(null);
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
  const handleReconGraphProgress = useCallback(
    (event: ReconGraphProgressDto) => {
      const previous = streamedGraphRef.current;
      if (previous !== null && previous.requestId === event.requestId) {
        if (event.sequence <= previous.sequence) return;
      }
      const base: ReconGraphDto =
        previous !== null && previous.requestId === event.requestId
          ? previous.graph
          : {
              schemaVersion: '1.4.0',
              assets: [],
              evidence: [],
              relationships: [],
              unmappedEventTypes: {},
            };
      const appendUnique = <T extends { id: string }>(
        current: T[],
        incoming: T[],
      ) => {
        const ids = new Set(current.map(({ id }) => id));
        return [...current, ...incoming.filter(({ id }) => !ids.has(id))];
      };
      const graph: ReconGraphDto = {
        ...base,
        assets: appendUnique(base.assets, event.assets),
        evidence: appendUnique(base.evidence, event.evidence),
        relationships: appendUnique(base.relationships, event.relationships),
        unmappedEventTypes:
          event.phase === 'completed'
            ? event.unmappedEventTypes
            : base.unmappedEventTypes,
      };
      streamedGraphRef.current = {
        requestId: event.requestId,
        sequence: event.sequence,
        graph,
      };
      setStreamedGraphRequestId(event.requestId);
      setStreamedGraph(graph);
    },
    [],
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

  const statsGraph =
    streamedGraph ??
    (state.status === 'success' ? state.reconGraph : undefined);
  const reconStats =
    state.status === 'success'
      ? [
          {
            label: 'Activos',
            value: statsGraph?.assets.length ?? state.snapshot.assets.length,
          },
          {
            label: 'Relaciones',
            value:
              statsGraph?.relationships.length ??
              state.snapshot.relationships.length,
          },
          {
            label: 'Evidencias',
            value:
              statsGraph?.evidence.length ?? state.snapshot.evidence.length,
          },
          {
            label: 'Modo',
            value: client.runtimeMode === 'demo' ? 'Offline' : 'Conectado',
          },
        ]
      : [];

  function navigateTo(section: string, targetId: string) {
    setActiveWorkspace(section);
    void targetId;
  }

  return (
    <AmbientHueContext.Provider value={accentHue}>
      <div className="chef-shell">
        <AuroraBackground preset={ambient} />
        {activeWorkspace === 'recon' && (
          <GlobeStage preset={ambient} progressRef={planetProgress} />
        )}
        <aside className="project-sidebar" aria-label="Navegación de CHEF">
          <a className="wordmark" href="#workspace" aria-label="CHEF, inicio">
            <img src="/chef-mascot.webp" alt="" />
            <span>CHEF</span>
          </a>
          <div className="project-switcher">
            <span className="project-avatar" aria-hidden="true">
              <img src="/chef-mascot.webp" alt="" width="30" height="30" />
            </span>
            <span>
              <strong>{activeRunLabel}</strong>
              <small>
                {state.status === 'success'
                  ? 'Snapshot cargado'
                  : 'Sin snapshot'}
              </small>
            </span>
            <span className="switcher-chevron" aria-hidden="true">
              ⌄
            </span>
          </div>
          <p className="sidebar-label">Espacio de trabajo</p>
          <nav className="workspace-nav" aria-label="Vistas del proyecto">
            <button
              className={
                activeWorkspace === 'recon' ? 'nav-item is-active' : 'nav-item'
              }
              type="button"
              aria-current={activeWorkspace === 'recon' ? 'page' : undefined}
              onClick={() => navigateTo('recon', 'overview')}
            >
              <WorkspaceIcon name="recon" /> Reconocimiento
            </button>
            <button
              className={
                activeWorkspace === 'evidence'
                  ? 'nav-item is-active'
                  : 'nav-item'
              }
              type="button"
              aria-current={activeWorkspace === 'evidence' ? 'page' : undefined}
              onClick={() => navigateTo('evidence', 'evidence-workspace')}
            >
              <WorkspaceIcon name="evidence" /> Evidencia
            </button>
            <button
              className={
                activeWorkspace === 'sources'
                  ? 'nav-item is-active'
                  : 'nav-item'
              }
              type="button"
              aria-current={activeWorkspace === 'sources' ? 'page' : undefined}
              onClick={() => navigateTo('sources', 'source-imports')}
            >
              <WorkspaceIcon name="sources" /> Fuentes
            </button>
            <button
              className={
                activeWorkspace === 'settings'
                  ? 'nav-item is-active'
                  : 'nav-item'
              }
              type="button"
              aria-current={activeWorkspace === 'settings' ? 'page' : undefined}
              onClick={() => navigateTo('settings', 'chef-config-title')}
            >
              <WorkspaceIcon name="settings" /> Ajustes
            </button>
          </nav>
          <section
            className="command-rail"
            aria-label="Comandos de la terminal"
          >
            <p className="sidebar-label">Comandos</p>
            <ul>
              <li>
                <code>chef help</code>
                <span>ayuda</span>
              </li>
              <li>
                <code>chef status</code>
                <span>estado</span>
              </li>
              <li>
                <code>chef url</code>
                <span>clasifica</span>
              </li>
              <li>
                <code>chef report</code>
                <span>informe</span>
              </li>
              <li>
                <code>clear</code>
                <span>limpiar</span>
              </li>
            </ul>
            <p className="rail-note">
              chef url clasifica la URL localmente; no consulta la red.
            </p>
          </section>
        </aside>
        <main id="workspace" className="chef-main">
          <header className="topbar">
            <div className="breadcrumbs">
              <span>CHEF</span>
              <b>/</b>
              <strong>{activeRunLabel}</strong>
            </div>
            <div className="topbar-actions">
              <label className="ambient-picker">
                <span>Ambiente</span>
                <select
                  value={ambient}
                  onChange={(event) =>
                    setAmbient(event.target.value as AuroraPreset)
                  }
                >
                  {AURORA_PRESETS.map((preset) => (
                    <option key={preset} value={preset}>
                      {auroraLabel(preset)}
                    </option>
                  ))}
                </select>
              </label>
              <span
                className={`backend-status ${backendStatusClass}`}
                aria-live="polite"
              >
                <i aria-hidden="true" /> {backendStatus}
              </span>
            </div>
          </header>
          <div
            className={
              activeWorkspace === 'recon'
                ? 'main-content recon-content'
                : 'main-content'
            }
          >
            <section className="page-heading" id="overview">
              <div>
                <h1>
                  {activeWorkspace === 'recon'
                    ? 'Reconocimiento'
                    : activeWorkspace === 'evidence'
                      ? 'Evidencia'
                      : activeWorkspace === 'sources'
                        ? 'Importaciones'
                        : 'Configuración'}
                </h1>
                <p className="page-description">
                  {activeWorkspace === 'recon'
                    ? 'Explora activos y evidencia del snapshot cargado.'
                    : activeWorkspace === 'evidence'
                      ? 'Consulta procedencia, relaciones y exportaciones.'
                      : activeWorkspace === 'sources'
                        ? 'Importa archivos de evidencia para procesarlos en CHEF.'
                        : 'Consulta el alcance y el estado de CHEF.'}
                </p>
              </div>
              <div className="workspace-badge">
                <span className="status-dot" />{' '}
                {state.status === 'success'
                  ? `${state.snapshot.assets.length} en snapshot`
                  : 'Sin snapshot'}
              </div>
            </section>
            <div
              className="recon-workbench"
              hidden={activeWorkspace !== 'recon'}
            >
              <div className="graph-column">
                {streamedGraph !== null && (
                  <p role="status" className="graph-status">
                    Mostrando el grafo ya validado y guardado · cargando su
                    representación
                  </p>
                )}
                {state.status === 'success' &&
                  (streamedGraph ?? state.reconGraph) !== undefined &&
                  ((streamedGraph ?? state.reconGraph)!.assets.length > 0 ? (
                    <ReconGraphPanel
                      variant="open"
                      key={
                        streamedGraphRequestId ??
                        `snapshot:${state.snapshot.run.id}`
                      }
                      graph={streamedGraph ?? state.reconGraph!}
                    />
                  ) : (
                    <p className="pane-empty">
                      Aún no hay activos proyectados.
                    </p>
                  ))}
                {state.status === 'success' &&
                  streamedGraph === null &&
                  state.reconGraph === undefined && (
                    <p className="pane-empty">
                      El snapshot no contiene una proyección de grafo.
                    </p>
                  )}
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
                {reconStats.length > 0 && (
                  <aside className="summary-rail" aria-label="Resumen del mapa">
                    <ul>
                      {reconStats.map((stat, index) => (
                        <li key={stat.label} className="stat-card">
                          <span>{stat.label}</span>
                          <strong>
                            {typeof stat.value === 'number' ? (
                              <CountUp
                                value={stat.value}
                                delay={260 + index * 110}
                              />
                            ) : (
                              stat.value
                            )}
                          </strong>
                        </li>
                      ))}
                    </ul>
                  </aside>
                )}
              </div>
              <div
                className={
                  consoleOpen
                    ? 'terminal-column'
                    : 'terminal-column is-collapsed'
                }
              >
                <h2>
                  <button
                    type="button"
                    className="console-toggle"
                    aria-expanded={consoleOpen}
                    aria-controls="activity-terminal"
                    onClick={() => setConsoleOpen((open) => !open)}
                  >
                    Consola
                  </button>
                </h2>
                <Suspense
                  fallback={
                    <section
                      aria-label="Terminal CHEF"
                      className="scan-log-panel terminal-loading"
                    >
                      Cargando terminal…
                    </section>
                  }
                >
                  <div id="activity-terminal">
                    <ScanLogTerminal
                      logs={scanLogs}
                      onCommand={runChefCommand}
                    />
                  </div>
                </Suspense>
              </div>
            </div>
            <section
              aria-labelledby="chef-config-title"
              id="settings-panel"
              className="config-strip"
              hidden={activeWorkspace !== 'settings'}
            >
              <h2 id="chef-config-title">Configuración local</h2>
              {configLoadFailed && (
                <p>No se pudo cargar config.toml de CHEF.</p>
              )}
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
            <div
              id="source-imports"
              className="imports-workspace"
              hidden={activeWorkspace !== 'sources'}
            >
              {client.importBbotJsonl !== undefined &&
                state.status === 'success' && (
                  <EvidenceImportPanel
                    client={client}
                    onImported={handleBbotImported}
                    onGraphProgress={handleReconGraphProgress}
                  />
                )}
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
                            .map(
                              ([eventType, count]) => `${eventType}: ${count}`,
                            )
                            .join(', ')}
                    </dd>
                  </dl>
                  <p>
                    Las relaciones y activos de esta importación son pistas de
                    un snapshot histórico; no representan estado actual.
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
            {state.status === 'success' &&
              censysImport === null &&
              urlscanImport === null && (
                <>
                  <section
                    aria-labelledby="snapshot-summary-title"
                    id="snapshot-summary"
                    className="snapshot-summary"
                    hidden={activeWorkspace !== 'evidence'}
                  >
                    <h2 id="snapshot-summary-title">Resumen del snapshot</h2>
                    <dl>
                      <div className="summary-metric">
                        <dt>Versión</dt>
                        <dd>{state.snapshot.schemaVersion}</dd>
                      </div>
                      <div className="summary-metric">
                        <dt>Activos</dt>
                        <dd>{state.snapshot.assets.length}</dd>
                      </div>
                      <div className="summary-metric">
                        <dt>Observaciones</dt>
                        <dd>{state.snapshot.observations.length}</dd>
                      </div>
                      <div className="summary-metric">
                        <dt>Evidencias</dt>
                        <dd>{state.snapshot.evidence.length}</dd>
                      </div>
                    </dl>
                  </section>
                  {activeWorkspace === 'evidence' &&
                    client.exportSnapshot !== undefined && (
                      <SnapshotExportButton
                        client={client}
                        snapshot={state.snapshot}
                        hasReconGraph={state.reconGraph !== undefined}
                      />
                    )}
                  {activeWorkspace === 'evidence' &&
                    eventStatistics !== null && (
                      <EventStatisticsPanel statistics={eventStatistics} />
                    )}
                  <div
                    className="explorer-grid"
                    id="evidence-workspace"
                    hidden={activeWorkspace !== 'evidence'}
                  >
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
            <footer className="legal-notice" aria-label="Licencia y atribución">
              <details>
                <summary>Licencia y avisos</summary>
                <p>
                  CHEF se distribuye bajo AGPL-3.0-only. Copyright © Black
                  Lantern Security y contribuidores de CHEF. El programa se
                  ofrece sin garantía; puedes redistribuirlo conforme a los
                  términos de la licencia.
                </p>
                <p>
                  Consulta los{' '}
                  <a href="/LICENSE" target="_blank" rel="noreferrer">
                    términos íntegros de AGPL-3.0
                  </a>{' '}
                  y los <a href="/THIRD_PARTY_NOTICES.md">avisos de terceros</a>
                  .
                </p>
              </details>
            </footer>
            {activeWorkspace === 'recon' && (
              <GlobeSpacer progressRef={planetProgress} />
            )}
          </div>
        </main>
      </div>
    </AmbientHueContext.Provider>
  );
}

function safeDownloadName(value: string) {
  const name = Array.from(value, (character) =>
    /[\\/:*?"<>|]/.test(character) || character < ' ' ? '-' : character,
  )
    .join('')
    .trim();
  return name.length > 0 ? name : 'chef-informe.html';
}

function WorkspaceIcon({
  name,
}: {
  name: 'recon' | 'evidence' | 'sources' | 'settings';
}) {
  const paths = {
    recon: (
      <>
        <circle cx="10.5" cy="10.5" r="6.5" />
        <path d="m15.5 15.5 5 5M10.5 7v7M7 10.5h7" />
      </>
    ),
    evidence: (
      <>
        <path d="M5 3.5h10l4 4V20.5H5z" />
        <path d="M15 3.5v5h4M8 12h8M8 16h8" />
      </>
    ),
    sources: (
      <>
        <path d="M12 3v12M7 10l5 5 5-5M4 20h16" />
      </>
    ),
    settings: (
      <>
        <circle cx="12" cy="12" r="3" />
        <path d="m19.4 15 .1.1 1.2 2-2 2-2-1.2-.2-.1-1.6.7-.1.2-.3 2.3h-2.9l-.3-2.3-.1-.2-1.6-.7-.2.1-2 1.2-2-2 1.2-2 .1-.2-.7-1.6-.2-.1L3 12.8V10l2.3-.3.2-.1.7-1.6-.1-.2-1.2-2 2-2 2 1.2.2.1 1.6-.7.1-.2L11.1 2h2.8l.3 2.3.1.2 1.6.7.2-.1 2-1.2 2 2-1.2 2-.1.2.7 1.6.2.1L22 10v2.8l-2.3.3-.2.1-.7 1.6z" />
      </>
    ),
  } satisfies Record<typeof name, ReactNode>;
  return (
    <svg
      aria-hidden="true"
      className="workspace-icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {paths[name]}
    </svg>
  );
}
