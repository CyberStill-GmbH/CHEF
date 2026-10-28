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
  const selectedAssetIdRef = useRef(selectedAssetId);

  useEffect(() => {
    let disposed = false;
    let cy: Core | null = null;
    const container = document.getElementById('recon-graph-canvas');
    if (container === null) return;

    const elements: ElementDefinition[] = [
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

    void import('cytoscape').then(({ default: cytoscape }) => {
      if (disposed) return;
      cy = cytoscape({
        container,
        elements,
        layout: { name: 'cose', animate: false, fit: true, padding: 24 },
        minZoom: 0.2,
        maxZoom: 3,
        style: [
          {
            selector: 'node',
            style: {
              label: 'data(label)',
              'text-wrap': 'wrap',
              'text-max-width': '140px',
              'font-size': 10,
              color: '#e2e8f0',
              'text-valign': 'center',
              'text-halign': 'center',
              width: 150,
              height: 54,
              shape: 'round-rectangle',
              'background-color': '#1e293b',
              'border-width': 2,
              'border-color': '#38bdf8',
            },
          },
          {
            selector: 'node.candidate',
            style: { 'background-color': '#422006', 'border-color': '#f59e0b' },
          },
          {
            selector: 'node.hint',
            style: {
              'background-color': '#312e1b',
              'border-color': '#eab308',
              'border-style': 'dashed',
            },
          },
          {
            selector: 'node.unresolved',
            style: {
              'background-color': '#27272a',
              'border-color': '#a1a1aa',
              'border-style': 'dashed',
            },
          },
          {
            selector: 'edge',
            style: {
              label: 'data(label)',
              'font-size': 8,
              color: '#94a3b8',
              'text-background-color': '#0f172a',
              'text-background-opacity': 0.9,
              'text-background-padding': '2px',
              'curve-style': 'bezier',
              'target-arrow-shape': 'triangle',
              'line-color': '#64748b',
              'target-arrow-color': '#64748b',
              width: 1.5,
            },
          },
          {
            selector: 'edge.hint',
            style: {
              'line-style': 'dashed',
              'line-color': '#eab308',
              'target-arrow-color': '#eab308',
            },
          },
          {
            selector: 'node:selected',
            style: { 'border-width': 4, 'border-color': '#a78bfa' },
          },
        ],
      });
      graphRef.current = cy;
      if (selectedAssetIdRef.current !== null) {
        cy.getElementById(selectedAssetIdRef.current).select();
      }
      cy.on('tap', 'node', (event) => setSelectedAssetId(event.target.id()));
    });

    return () => {
      disposed = true;
      graphRef.current = null;
      cy?.destroy();
    };
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
      <h2 id="recon-graph-title">Grafo de reconocimiento CHEF</h2>
      <p>
        {graph.assets.length} activos · {graph.relationships.length} relaciones
        · {graph.evidence.length} evidencias
      </p>
      <p>
        El ámbar marca hallazgos candidatos. Las relaciones pueden ser pistas
        sin verificar.
      </p>
      <div
        id="recon-graph-canvas"
        className="recon-graph-canvas"
        role="img"
        aria-label="Diagrama de activos y relaciones. Usa la lista accesible para seleccionar un activo y consultar su evidencia."
      />

      <label htmlFor="recon-asset-filter">Filtrar activos</label>
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
