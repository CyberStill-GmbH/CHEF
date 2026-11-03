import { useContext, useEffect, useMemo, useRef, useState } from 'react';
import type {
  Core,
  ElementDefinition,
  NodeSingular,
  StylesheetJson,
} from 'cytoscape';

import type { ReconAssetDto, ReconGraphDto } from '../contracts/recon-graph';
import { AmbientHueContext, hslToHex } from '../ambient';
import { prefersReducedMotion } from '../motion';

interface ReconGraphPanelProps {
  /** 'open' = mapa sin caja con paneles flotantes (Reconocimiento); 'embedded' = caja apilada. */
  variant?: 'open' | 'embedded';
  graph: ReconGraphDto;
}

const KIND_LABELS: Record<ReconAssetDto['kind'], string> = {
  domain: 'Dominio',
  ip_address: 'Dirección IP',
  service: 'Servicio',
  web_endpoint: 'Endpoint web',
  technology: 'Tecnología',
  certificate: 'Certificado',
  candidate_finding: 'Hallazgo candidato',
  organization: 'Organización sugerida',
};

const MAX_VISIBLE_ASSETS = 200;

const RELATIONSHIP_LABELS: Record<string, string> = {
  parent_domain_of: 'dominio padre sugerido',
  exposes_service: 'servicio TCP asociado',
};

const ASSERTION_LABELS: Record<string, string> = {
  observed: 'observada',
  candidate: 'candidata',
  unverified: 'sin verificar',
  hint: 'pista',
  unresolved: 'sin resolver',
  observed_open: 'reportado abierto',
};

/** Orden de los tipos al repartir los vecinos de un nodo. */
const KIND_ORDER: Record<ReconAssetDto['kind'], number> = {
  organization: 0,
  domain: 1,
  ip_address: 2,
  service: 3,
  web_endpoint: 4,
  technology: 5,
  certificate: 6,
  candidate_finding: 7,
};

/** Separación entre niveles: deja sitio a la etiqueta de cada relación. */
const LEVEL_GAP = 460;
const COLUMN_GAP = 300;
const ROW_GAP = 84;

/**
 * Dispone el mapa como un abanico: en cada grupo conectado el activo con más
 * relaciones queda a la izquierda y sus vecinos se reparten en una columna a su
 * derecha, con sitio para las etiquetas; los siguientes niveles continúan hacia
 * la derecha. Así las líneas parten de un punto y no atraviesan otros nodos. Los
 * nodos sin relaciones van en una banda propia debajo.
 */
function arrangeGraph(cy: Core) {
  const lonely = cy.nodes().filter((node) => node.degree(false) === 0);
  let offsetY = 0;
  cy.elements()
    .components()
    .filter((component) => component.nodes().length > 1)
    .forEach((component) => {
      const nodes = component.nodes().toArray();
      const hub = nodes.reduce((best, node) =>
        node.degree(false) > best.degree(false) ? node : best,
      );
      const levels: NodeSingular[][] = [[hub]];
      const seen = new Set([hub.id()]);
      for (let depth = 0; depth < levels.length; depth += 1) {
        const next: NodeSingular[] = [];
        for (const node of levels[depth] ?? []) {
          node.neighborhood('node').forEach((neighbor) => {
            if (seen.has(neighbor.id())) return;
            seen.add(neighbor.id());
            next.push(neighbor);
          });
        }
        if (next.length > 0) levels.push(next);
      }
      const height = Math.max(...levels.map((level) => level.length)) * ROW_GAP;
      const placed = new Map<string, number>();
      levels.forEach((level, depth) => {
        const score = (node: NodeSingular) => {
          const ys = node
            .neighborhood('node')
            .map((neighbor) => placed.get(neighbor.id()))
            .filter((y): y is number => y !== undefined);
          return ys.length === 0
            ? 0
            : ys.reduce((sum, y) => sum + y, 0) / ys.length;
        };
        const order = (node: NodeSingular) =>
          KIND_ORDER[node.data('kind') as ReconAssetDto['kind']];
        level.sort((a, b) =>
          depth <= 1
            ? order(a) - order(b) || a.id().localeCompare(b.id())
            : score(a) - score(b),
        );
        level.forEach((node, row) => {
          const y =
            offsetY + height / 2 + (row - (level.length - 1) / 2) * ROW_GAP;
          placed.set(node.id(), y);
          node.position({ x: depth * LEVEL_GAP, y });
        });
      });
      offsetY += height + 90;
    });
  const columns = 3;
  lonely.forEach((node, index) => {
    node.position({
      x: (index % columns) * COLUMN_GAP,
      y: offsetY + 30 + Math.floor(index / columns) * ROW_GAP,
    });
  });
}

/** Hoja de estilo del grafo: el acento sale del matiz del ambiente activo. */
function graphStyle(hue: number): StylesheetJson {
  return [
    {
      selector: 'node',
      style: {
        label: 'data(label)',
        'text-wrap': 'wrap',
        'text-max-width': '214px',
        'font-size': 15,
        'font-family': "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
        color: '#e4e6e8',
        'text-valign': 'center',
        'text-halign': 'center',
        width: 240,
        height: 58,
        shape: 'round-rectangle',
        'corner-radius': '29',
        'background-color': hslToHex(hue, 14, 7),
        'border-width': 1.5,
        'border-color': hslToHex(hue, 45, 32),
      },
    },
    {
      selector: 'node.candidate',
      style: { 'background-color': '#17130a', 'border-color': '#f2b35a' },
    },
    {
      selector: 'node.hint',
      style: {
        'background-color': '#17130a',
        'border-color': '#f2b35a',
        'border-style': 'dashed',
      },
    },
    {
      selector: 'node.unresolved',
      style: {
        'background-color': '#14161a',
        'border-color': '#8b9097',
        'border-style': 'dashed',
      },
    },
    {
      selector: 'edge',
      style: {
        label: 'data(label)',
        'font-size': 12,
        'text-margin-y': -9,
        'font-family': "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
        color: '#8b9097',
        'text-background-color': '#0a0b0d',
        'text-background-opacity': 0.9,
        'text-background-padding': '2px',
        'curve-style': 'bezier',
        'target-arrow-shape': 'triangle',
        'line-color': hslToHex(hue, 30, 46),
        'target-arrow-color': hslToHex(hue, 30, 46),
        width: 2.5,
        'arrow-scale': 1.3,
      },
    },
    {
      selector: 'edge.hint',
      style: {
        'line-style': 'dashed',
        'line-color': '#f2b35a',
        'target-arrow-color': '#f2b35a',
      },
    },
    {
      selector: 'node:selected',
      style: {
        'border-width': 2.5,
        'border-color': hslToHex(hue, 94, 60),
        'background-color': hslToHex(hue, 40, 9),
      },
    },
  ];
}
export function ReconGraphPanel({
  graph,
  variant = 'embedded',
}: ReconGraphPanelProps) {
  const [assetsOpen, setAssetsOpen] = useState(false);
  const [filter, setFilter] = useState('');
  const [selectedAssetId, setSelectedAssetId] = useState<string | null>(
    variant === 'open' ? null : (graph.assets[0]?.id ?? null),
  );
  const graphRef = useRef<Core | null>(null);
  const latestGraphRef = useRef(graph);
  const selectedAssetIdRef = useRef(selectedAssetId);
  latestGraphRef.current = graph;
  const hue = useContext(AmbientHueContext);
  const hueRef = useRef(hue);
  hueRef.current = hue;

  // Al cambiar de ambiente, el grafo se repinta con el nuevo acento.
  useEffect(() => {
    graphRef.current?.style(graphStyle(hue));
  }, [hue]);

  useEffect(() => {
    let disposed = false;
    let cy: Core | null = null;
    const container = document.getElementById('recon-graph-canvas');
    if (container === null) return;
    const observer = new ResizeObserver(() => {
      if (
        cy === null ||
        container.clientWidth === 0 ||
        container.clientHeight === 0
      )
        return;
      cy.resize();
      cy.fit(undefined, 24);
    });
    observer.observe(container);

    void import('cytoscape').then(({ default: cytoscape }) => {
      if (disposed) return;
      const initialGraph = latestGraphRef.current;
      cy = cytoscape({
        container,
        elements: graphElements(initialGraph),
        layout: { name: 'preset' },
        // Mapa fijo, como una imagen: sin zoom, sin arrastre y sin mover nodos,
        // para que la rueda del ratón siga desplazando la página.
        userZoomingEnabled: false,
        userPanningEnabled: false,
        boxSelectionEnabled: false,
        autoungrabify: true,
        minZoom: 0.2,
        maxZoom: 3,
        style: graphStyle(hueRef.current),
      });
      graphRef.current = cy;
      arrangeGraph(cy);
      if (selectedAssetIdRef.current !== null) {
        cy.getElementById(selectedAssetIdRef.current).select();
      }
      if (container.clientWidth > 0 && container.clientHeight > 0) {
        cy.resize();
        cy.fit(undefined, 28);
      }
      cy.on('tap', 'node', (event) => setSelectedAssetId(event.target.id()));

      // Aparición animada: los nodos brotan desde el centro hacia su lugar, en
      // cascada, y después se dibujan las relaciones. Solo es presentación; el
      // layout final ya está calculado.
      if (!prefersReducedMotion()) {
        const nodes = cy.nodes();
        const edges = cy.edges();
        const extent = cy.extent();
        const centerX = (extent.x1 + extent.x2) / 2;
        const centerY = (extent.y1 + extent.y2) / 2;
        const destinations = new Map(
          nodes.map((node) => [node.id(), { ...node.position() }] as const),
        );
        nodes.style('opacity', 0);
        edges.style('opacity', 0);
        const lastIndex = nodes.length - 1;
        nodes.forEach((node, index) => {
          node.position({ x: centerX, y: centerY });
          node.delay(index * 70).animate(
            {
              position: destinations.get(node.id()) ?? node.position(),
              style: { opacity: 1 },
            },
            {
              duration: 750,
              easing: 'ease-out-cubic',
              // Un reajuste de tamaño durante la animación encuadra los nodos
              // aún colapsados en el centro; al terminar se recoloca la cámara.
              ...(index === lastIndex
                ? {
                    complete: () => {
                      if (cy?.destroyed()) return;
                      cy?.animate(
                        { fit: { eles: cy.elements(), padding: 28 } },
                        { duration: 450, easing: 'ease-out-cubic' },
                      );
                    },
                  }
                : {}),
            },
          );
        });
        edges.forEach((edge, index) => {
          edge
            .delay(nodes.length * 70 + 250 + index * 50)
            .animate(
              { style: { opacity: 1 } },
              { duration: 500, easing: 'ease-out-cubic' },
            );
        });
      }
    });

    return () => {
      disposed = true;
      observer.disconnect();
      graphRef.current = null;
      cy?.destroy();
    };
  }, []);

  useEffect(() => {
    const cy = graphRef.current;
    if (cy === null) return;

    const missingAssets = graph.assets.filter(({ id }) =>
      cy.getElementById(id).empty(),
    );
    const missingRelationships = graph.relationships.filter(
      ({ id, fromAssetId, toAssetId }) =>
        cy.getElementById(id).empty() &&
        !cy.getElementById(fromAssetId).empty() &&
        !cy.getElementById(toAssetId).empty(),
    );
    if (missingAssets.length === 0 && missingRelationships.length === 0) return;

    const additions: ElementDefinition[] = [
      ...graphElements({
        ...graph,
        assets: missingAssets,
        relationships: missingRelationships,
      }),
    ];
    const added = cy.add(additions);
    const reducedMotion = window.matchMedia(
      '(prefers-reduced-motion: reduce)',
    ).matches;
    added.layout({
      name: 'cose',
      animate: !reducedMotion,
      fit: false,
      randomize: false,
      animationDuration: 360,
    });
    if (!reducedMotion) {
      const newNodes = added.filter('node');
      newNodes.style('opacity', 0);
      newNodes.animate({ style: { opacity: 1 } }, { duration: 320 });
    }
  }, [graph.assets, graph.relationships]);

  useEffect(() => {
    selectedAssetIdRef.current = selectedAssetId;
    const cy = graphRef.current;
    if (cy === null) return;
    cy.nodes().unselect();
    if (selectedAssetId !== null) cy.getElementById(selectedAssetId).select();
  }, [selectedAssetId]);

  useEffect(() => {
    if (
      selectedAssetId === null &&
      graph.assets.length > 0 &&
      variant !== 'open'
    ) {
      setSelectedAssetId(graph.assets[0]?.id ?? null);
    } else if (
      selectedAssetId !== null &&
      !graph.assets.some(({ id }) => id === selectedAssetId)
    ) {
      setSelectedAssetId(
        variant === 'open' ? null : (graph.assets[0]?.id ?? null),
      );
    }
  }, [graph.assets, selectedAssetId, variant]);

  const filteredAssets = useMemo(() => {
    const normalizedFilter = filter.trim().toLocaleLowerCase();
    const matching = graph.assets.filter(
      ({ key, kind }) =>
        normalizedFilter.length === 0 ||
        key.toLocaleLowerCase().includes(normalizedFilter) ||
        KIND_LABELS[kind].toLocaleLowerCase().includes(normalizedFilter),
    );
    return {
      matchingCount: matching.length,
      visible: matching.slice(0, MAX_VISIBLE_ASSETS),
    };
  }, [filter, graph.assets]);

  const selectedAsset = graph.assets.find(({ id }) => id === selectedAssetId);
  const selectedEvidence = graph.evidence.filter(({ id }) =>
    selectedAsset?.evidenceIds.includes(id),
  );
  const connectedRelationships = graph.relationships.filter(
    ({ fromAssetId, toAssetId }) =>
      fromAssetId === selectedAssetId || toAssetId === selectedAssetId,
  );

  if (variant === 'open') {
    return (
      <div
        className="graph-open"
        role="region"
        aria-labelledby="recon-graph-title"
      >
        <div
          id="recon-graph-canvas"
          className="recon-graph-canvas"
          role="img"
          aria-label="Diagrama de activos y relaciones. Usa la lista de activos para seleccionar uno y consultar su evidencia."
        />

        <div className="graph-hud graph-hud-left">
          <h2 id="recon-graph-title">Mapa de relaciones</h2>
          <p className="graph-counts">
            {graph.assets.length} activos · {graph.relationships.length}{' '}
            relaciones · {graph.evidence.length} evidencias
          </p>
          <div className="graph-tools">
            <label className="graph-sr-only" htmlFor="recon-asset-filter">
              Filtrar activos
            </label>
            <input
              id="recon-asset-filter"
              type="search"
              placeholder="Filtrar activos…"
              value={filter}
              onChange={(event) => setFilter(event.currentTarget.value)}
            />
            <button
              type="button"
              aria-expanded={assetsOpen}
              aria-controls="recon-asset-panel"
              onClick={() => setAssetsOpen((open) => !open)}
            >
              Activos · {filteredAssets.matchingCount}
            </button>
          </div>
          <div
            id="recon-asset-panel"
            className="graph-assets"
            hidden={!assetsOpen}
          >
            {filteredAssets.matchingCount === 0 ? (
              <p>No hay activos que coincidan con el filtro.</p>
            ) : (
              <>
                <ul className="recon-asset-list">
                  {filteredAssets.visible.map((asset) => (
                    <li key={asset.id}>
                      <button
                        type="button"
                        aria-pressed={asset.id === selectedAssetId}
                        onClick={() => setSelectedAssetId(asset.id)}
                      >
                        <span>{KIND_LABELS[asset.kind]}</span>
                        <strong>{asset.key}</strong>
                        {asset.qualification !== null && (
                          <span>
                            Estado:{' '}
                            {ASSERTION_LABELS[asset.qualification] ??
                              asset.qualification}
                          </span>
                        )}
                      </button>
                    </li>
                  ))}
                </ul>
                {filteredAssets.matchingCount > MAX_VISIBLE_ASSETS && (
                  <p>
                    Mostrando {MAX_VISIBLE_ASSETS} de{' '}
                    {filteredAssets.matchingCount}; ajusta el filtro para
                    encontrar otros activos.
                  </p>
                )}
              </>
            )}
          </div>
        </div>

        <aside
          className="graph-hud graph-inspector"
          aria-label="Detalle del activo seleccionado"
        >
          {selectedAsset === undefined ? (
            <p className="graph-hint">
              Selecciona un nodo para revisar su evidencia y relaciones.
            </p>
          ) : (
            <>
              <button
                type="button"
                className="inspector-close"
                aria-label="Cerrar detalle"
                onClick={() => setSelectedAssetId(null)}
              >
                ✕
              </button>
              <p className="inspector-kind">
                {KIND_LABELS[selectedAsset.kind]}
              </p>
              <h3 id="recon-selection-title" className="inspector-key">
                {selectedAsset.key}
              </h3>
              <dl className="inspector-facts">
                <div>
                  <dt>Identificador CHEF</dt>
                  <dd>{selectedAsset.id}</dd>
                </div>
                <div>
                  <dt>Observado por primera vez</dt>
                  <dd>{selectedAsset.firstSeen}</dd>
                </div>
                <div>
                  <dt>Observado por última vez</dt>
                  <dd>{selectedAsset.lastSeen}</dd>
                </div>
              </dl>
              <details className="inspector-group" open>
                <summary>
                  Evidencia de origen <span>{selectedEvidence.length}</span>
                </summary>
                {selectedEvidence.length === 0 ? (
                  <p>No hay evidencia asociada a este activo.</p>
                ) : (
                  selectedEvidence.map((item) => (
                    <details key={item.id} className="inspector-item">
                      <summary>{item.eventType}</summary>
                      <dl>
                        <dt>Fecha</dt>
                        <dd>{item.observedAt}</dd>
                        <dt>Localizador</dt>
                        <dd>{item.locator}</dd>
                        <dt>Módulo de origen</dt>
                        <dd>{item.module ?? 'No indicado'}</dd>
                        <dt>Redactado</dt>
                        <dd>{item.redacted ? 'Sí' : 'No'}</dd>
                      </dl>
                    </details>
                  ))
                )}
              </details>
              <details className="inspector-group">
                <summary>
                  Relaciones sustentadas{' '}
                  <span>{connectedRelationships.length}</span>
                </summary>
                {connectedRelationships.length === 0 ? (
                  <p>No hay relaciones sustentadas para este activo.</p>
                ) : (
                  connectedRelationships.map((relationship) => {
                    const from = graph.assets.find(
                      ({ id }) => id === relationship.fromAssetId,
                    );
                    const to = graph.assets.find(
                      ({ id }) => id === relationship.toAssetId,
                    );
                    return (
                      <details key={relationship.id} className="inspector-item">
                        <summary>
                          {from?.key ?? relationship.fromAssetId} →{' '}
                          {to?.key ?? relationship.toAssetId}
                        </summary>
                        <dl>
                          <dt>Tipo</dt>
                          <dd>
                            {RELATIONSHIP_LABELS[relationship.kind] ??
                              relationship.kind}
                          </dd>
                          <dt>Afirmación</dt>
                          <dd>
                            {ASSERTION_LABELS[relationship.assertion] ??
                              relationship.assertion}
                          </dd>
                        </dl>
                      </details>
                    );
                  })
                )}
              </details>
            </>
          )}
        </aside>

        <ul className="graph-hud graph-legend" aria-label="Leyenda del mapa">
          <li>
            <i className="legend-dot is-observed" />
            Observado
          </li>
          <li>
            <i className="legend-dot is-candidate" />
            Candidato
          </li>
          <li>
            <i className="legend-dot is-hint" />
            Pista
          </li>
          <li>
            <i className="legend-dot is-unresolved" />
            Sin resolver
          </li>
        </ul>

        <p className="graph-disclaimer">
          Haz clic en un nodo para ver su evidencia. La animación organiza
          evidencia importada; no representa tráfico en vivo. Los candidatos y
          las pistas no son vulnerabilidades confirmadas.
        </p>
      </div>
    );
  }
  return (
    <section aria-labelledby="recon-graph-title">
      <header className="graph-summary">
        <div>
          <h2 id="recon-graph-title">Mapa de relaciones</h2>
          <p>
            {graph.assets.length} activos · {graph.relationships.length}{' '}
            relaciones · {graph.evidence.length} evidencias
          </p>
        </div>
        <span className="graph-data-state">Snapshot local</span>
      </header>
      <p className="graph-disclaimer">
        La animación organiza evidencia importada; no representa tráfico en
        vivo. Los candidatos y las pistas no son vulnerabilidades confirmadas.
      </p>
      <div
        id="recon-graph-canvas"
        className="recon-graph-canvas"
        role="img"
        aria-label="Diagrama de activos y relaciones. Usa la lista accesible para seleccionar un activo y consultar su evidencia."
      />

      <label className="graph-filter-label" htmlFor="recon-asset-filter">
        Filtrar activos
      </label>
      <input
        id="recon-asset-filter"
        type="search"
        value={filter}
        onChange={(event) => setFilter(event.currentTarget.value)}
      />

      <div className="recon-explorer">
        <section aria-labelledby="recon-assets-title">
          <h3 id="recon-assets-title">Activos y candidatos</h3>
          {filteredAssets.matchingCount === 0 ? (
            <p>No hay activos que coincidan con el filtro.</p>
          ) : (
            <>
              <ul className="recon-asset-list">
                {filteredAssets.visible.map((asset) => (
                  <li key={asset.id}>
                    <button
                      type="button"
                      aria-pressed={asset.id === selectedAssetId}
                      onClick={() => setSelectedAssetId(asset.id)}
                    >
                      <span>{KIND_LABELS[asset.kind]}</span>
                      <strong>{asset.key}</strong>
                      {asset.qualification !== null && (
                        <span>
                          Estado:{' '}
                          {ASSERTION_LABELS[asset.qualification] ??
                            asset.qualification}
                        </span>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
              {filteredAssets.matchingCount > MAX_VISIBLE_ASSETS && (
                <p>
                  Mostrando {MAX_VISIBLE_ASSETS} de{' '}
                  {filteredAssets.matchingCount}; ajusta el filtro para
                  encontrar otros activos.
                </p>
              )}
            </>
          )}
        </section>

        <section aria-labelledby="recon-selection-title">
          <h3 id="recon-selection-title">Detalle y procedencia</h3>
          {selectedAsset === undefined ? (
            <p>Selecciona un activo para revisar su evidencia y relaciones.</p>
          ) : (
            <>
              <dl>
                <dt>Tipo</dt>
                <dd>{KIND_LABELS[selectedAsset.kind]}</dd>
                <dt>Identificador CHEF</dt>
                <dd>{selectedAsset.id}</dd>
                <dt>Observado por primera vez</dt>
                <dd>{selectedAsset.firstSeen}</dd>
                <dt>Observado por última vez</dt>
                <dd>{selectedAsset.lastSeen}</dd>
              </dl>

              <h4>Evidencia de origen</h4>
              {selectedEvidence.length === 0 ? (
                <p>No hay evidencia asociada a este activo.</p>
              ) : (
                <ul>
                  {selectedEvidence.map((item) => (
                    <li key={item.id}>
                      <dl>
                        <dt>Tipo de evento</dt>
                        <dd>{item.eventType}</dd>
                        <dt>Fecha</dt>
                        <dd>{item.observedAt}</dd>
                        <dt>Localizador</dt>
                        <dd>{item.locator}</dd>
                        <dt>Módulo de origen</dt>
                        <dd>{item.module ?? 'No indicado'}</dd>
                        <dt>Redactado</dt>
                        <dd>{item.redacted ? 'Sí' : 'No'}</dd>
                      </dl>
                    </li>
                  ))}
                </ul>
              )}

              <h4>Relaciones sustentadas</h4>
              {connectedRelationships.length === 0 ? (
                <p>No hay relaciones sustentadas para este activo.</p>
              ) : (
                <ul>
                  {connectedRelationships.map((relationship) => {
                    const from = graph.assets.find(
                      ({ id }) => id === relationship.fromAssetId,
                    );
                    const to = graph.assets.find(
                      ({ id }) => id === relationship.toAssetId,
                    );
                    return (
                      <li key={relationship.id}>
                        <p>
                          {from?.key ?? relationship.fromAssetId} →{' '}
                          {to?.key ?? relationship.toAssetId}
                        </p>
                        <dl>
                          <dt>Tipo</dt>
                          <dd>
                            {RELATIONSHIP_LABELS[relationship.kind] ??
                              relationship.kind}
                          </dd>
                          <dt>Afirmación</dt>
                          <dd>
                            {ASSERTION_LABELS[relationship.assertion] ??
                              relationship.assertion}
                          </dd>
                        </dl>
                      </li>
                    );
                  })}
                </ul>
              )}
            </>
          )}
        </section>
      </div>
    </section>
  );
}

function graphElements(graph: ReconGraphDto): ElementDefinition[] {
  return [
    ...graph.assets.map((asset) => ({
      data: {
        id: asset.id,
        label: `${KIND_LABELS[asset.kind]}\n${asset.key}`,
        kind: asset.kind,
      },
      classes:
        asset.kind === 'candidate_finding'
          ? 'candidate'
          : asset.qualification === 'hint'
            ? 'hint'
            : asset.qualification === 'unresolved'
              ? 'unresolved'
              : 'observed',
    })),
    ...graph.relationships.map((relationship) => ({
      data: {
        id: relationship.id,
        source: relationship.fromAssetId,
        target: relationship.toAssetId,
        label: RELATIONSHIP_LABELS[relationship.kind] ?? relationship.kind,
      },
      classes: relationship.assertion === 'hint' ? 'hint' : '',
    })),
  ];
}
