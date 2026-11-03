import { useEffect, useMemo, useRef, useState } from 'react';
import type { Core, ElementDefinition } from 'cytoscape';

import type { ReconAssetDto, ReconGraphDto } from '../contracts/recon-graph';

interface ReconGraphPanelProps {
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

export function ReconGraphPanel({ graph }: ReconGraphPanelProps) {
  const [filter, setFilter] = useState('');
  const [selectedAssetId, setSelectedAssetId] = useState<string | null>(
    graph.assets[0]?.id ?? null,
  );
  const graphRef = useRef<Core | null>(null);
  const latestGraphRef = useRef(graph);
  const selectedAssetIdRef = useRef(selectedAssetId);
  latestGraphRef.current = graph;

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
        layout: {
          name: 'cose',
          animate: false,
          fit: true,
          padding: 36,
          spacingFactor: 1.2,
          nodeRepulsion: 7200,
          idealEdgeLength: 170,
        },
        minZoom: 0.2,
        maxZoom: 3,
        style: [
          {
            selector: 'node',
            style: {
              label: 'data(label)',
              'text-wrap': 'wrap',
              'text-max-width': '140px',
              'font-size': 11,
              'font-family':
                "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
              color: '#e4e6e8',
              'text-valign': 'center',
              'text-halign': 'center',
              width: 156,
              height: 54,
              shape: 'round-rectangle',
              'background-color': '#0f1113',
              'border-width': 1.5,
              'border-color': '#2f5f48',
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
              'font-size': 9,
              'font-family':
                "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
              color: '#8b9097',
              'text-background-color': '#0a0b0d',
              'text-background-opacity': 0.9,
              'text-background-padding': '2px',
              'curve-style': 'bezier',
              'target-arrow-shape': 'triangle',
              'line-color': '#2e3338',
              'target-arrow-color': '#2e3338',
              width: 1.5,
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
              'border-color': '#5be3a0',
              'background-color': '#0f1a14',
            },
          },
        ],
      });
      graphRef.current = cy;
      if (selectedAssetIdRef.current !== null) {
        cy.getElementById(selectedAssetIdRef.current).select();
      }
      if (container.clientWidth > 0 && container.clientHeight > 0) {
        cy.resize();
        cy.fit(undefined, 24);
      }
      cy.on('tap', 'node', (event) => setSelectedAssetId(event.target.id()));
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
    if (selectedAssetId === null && graph.assets.length > 0) {
      setSelectedAssetId(graph.assets[0]?.id ?? null);
    } else if (
      selectedAssetId !== null &&
      !graph.assets.some(({ id }) => id === selectedAssetId)
    ) {
      setSelectedAssetId(graph.assets[0]?.id ?? null);
    }
  }, [graph.assets, selectedAssetId]);

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
