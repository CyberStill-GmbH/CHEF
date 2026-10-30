import { useEffect, useRef } from 'react';

import type { AssetDto, SnapshotDto } from '../contracts/snapshot';

interface EvidencePanelProps {
  snapshot: SnapshotDto;
  asset: AssetDto;
}

export function EvidencePanel({ snapshot, asset }: EvidencePanelProps) {
  const panelRef = useRef<HTMLElement>(null);

  // Al cambiar el asset seleccionado, mueve el foco al panel para que las
  // personas que navegan con teclado o lector de pantalla aterricen en el
  // contenido nuevo en lugar de quedarse en la lista.
  useEffect(() => {
    panelRef.current?.focus();
  }, [asset.id]);

  const observationReferences = asset.observationIds.map((observationId) => ({
    observationId,
    observation: snapshot.observations.find(({ id }) => id === observationId),
  }));
  const evidenceIds = [
    ...new Set(
      observationReferences.flatMap(({ observation }) =>
        observation === undefined ? [] : [observation.evidenceId],
      ),
    ),
  ];
  const evidenceReferences = evidenceIds.map((evidenceId) => ({
    evidenceId,
    evidence: snapshot.evidence.find(({ id }) => id === evidenceId),
  }));
  const relationships = snapshot.relationships.filter(
    ({ from, to }) => from === asset.id || to === asset.id,
  );
  const paths = snapshot.paths.filter(({ assetIds }) =>
    assetIds.includes(asset.id),
  );

  return (
    <aside
      ref={panelRef}
      tabIndex={-1}
      aria-labelledby="evidence-panel-title"
      aria-describedby="evidence-panel-description"
    >
      <p id="evidence-panel-description" className="visually-hidden">
        Panel de evidencia del asset seleccionado. Correlación offline y
        determinista: una vulnerabilidad solo se marca como confirmada cuando
        existe evidencia que la respalde.
      </p>
      <p className="visually-hidden" role="status">
        Mostrando evidencia del asset {asset.id} ({asset.kind}).
      </p>
      <section aria-labelledby="evidence-panel-title">
        <h2 id="evidence-panel-title">Asset seleccionado</h2>
        <dl>
          <dt>ID</dt>
          <dd>{asset.id}</dd>
          <dt>Kind</dt>
          <dd>{asset.kind}</dd>
          <dt>Key</dt>
          <dd>{asset.key}</dd>
          <dt>First seen</dt>
          <dd>{asset.firstSeen}</dd>
          <dt>Last seen</dt>
          <dd>{asset.lastSeen}</dd>
          <dt>Observation IDs</dt>
          <dd>{asset.observationIds.length}</dd>
        </dl>
      </section>

      <section aria-labelledby="related-observations-title">
        <h3 id="related-observations-title">Observaciones relacionadas</h3>
        {observationReferences.length === 0 ? (
          <p>Este asset no tiene observaciones referenciadas.</p>
        ) : (
          <ul>
            {observationReferences.map(
              ({ observationId, observation }, index) => (
                <li key={`${observationId}-${index}`}>
                  {observation === undefined ? (
                    <p>
                      La observación referenciada no está disponible:{' '}
                      {observationId}
                    </p>
                  ) : (
                    <section
                      aria-labelledby={`related-observation-${index}-title`}
                    >
                      <h4 id={`related-observation-${index}-title`}>
                        Observación {index + 1}
                      </h4>
                      <dl>
                        <dt>ID</dt>
                        <dd>{observation.id}</dd>
                        <dt>Fact</dt>
                        <dd>{observation.fact}</dd>
                        <dt>Source</dt>
                        <dd>{observation.source}</dd>
                        <dt>Observed at</dt>
                        <dd>{observation.observedAt}</dd>
                        <dt>Protocol</dt>
                        <dd>{observation.protocol}</dd>
                        <dt>Port</dt>
                        <dd>{observation.port}</dd>
                        <dt>Confidence</dt>
                        <dd>{observation.confidence}</dd>
                        <dt>Evidence ID</dt>
                        <dd>{observation.evidenceId}</dd>
                      </dl>
                    </section>
                  )}
                </li>
              ),
            )}
          </ul>
        )}
      </section>

      <section aria-labelledby="related-evidence-title">
        <h3 id="related-evidence-title">Evidence relacionada</h3>
        {evidenceReferences.length === 0 ? (
          <p>No hay evidence resoluble para este asset.</p>
        ) : (
          <ul>
            {evidenceReferences.map(({ evidenceId, evidence }, index) => (
              <li key={evidenceId}>
                {evidence === undefined ? (
                  <p>
                    La evidence referenciada no está disponible: {evidenceId}
                  </p>
                ) : (
                  <section aria-labelledby={`related-evidence-${index}-title`}>
                    <h4 id={`related-evidence-${index}-title`}>
                      Evidence {index + 1}
                    </h4>
                    <dl>
                      <dt>ID</dt>
                      <dd>{evidence.id}</dd>
                      <dt>SHA-256</dt>
                      <dd>{evidence.sha256}</dd>
                      <dt>Source</dt>
                      <dd>{evidence.source}</dd>
                      <dt>Parser version</dt>
                      <dd>{evidence.parserVersion}</dd>
                      <dt>Imported at</dt>
                      <dd>{evidence.importedAt}</dd>
                      <dt>Locator</dt>
                      <dd>{evidence.locator}</dd>
                    </dl>
                  </section>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section aria-labelledby="connected-relationships-title">
        <h3 id="connected-relationships-title">Relaciones conectadas</h3>
        {relationships.length === 0 ? (
          <p>Este asset no tiene relaciones conectadas.</p>
        ) : (
          <ul>
            {relationships.map((relationship, index) => (
              <li key={relationship.id}>
                <section
                  aria-labelledby={`connected-relationship-${index}-title`}
                >
                  <h4 id={`connected-relationship-${index}-title`}>
                    Relación {index + 1}
                  </h4>
                  <dl>
                    <dt>ID</dt>
                    <dd>{relationship.id}</dd>
                    <dt>Kind</dt>
                    <dd>{relationship.kind}</dd>
                    <dt>Assertion</dt>
                    <dd>{relationship.assertion}</dd>
                    <dt>Confidence</dt>
                    <dd>{relationship.confidence}</dd>
                    <dt>Source asset ID</dt>
                    <dd>{relationship.from}</dd>
                    <dt>Target asset ID</dt>
                    <dd>{relationship.to}</dd>
                  </dl>
                </section>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section aria-labelledby="related-paths-title">
        <h3 id="related-paths-title">Paths relacionados</h3>
        {paths.length === 0 ? (
          <p>Este asset no tiene paths relacionados.</p>
        ) : (
          <ul>
            {paths.map((path, index) => (
              <li key={path.id}>
                <section aria-labelledby={`related-path-${index}-title`}>
                  <h4 id={`related-path-${index}-title`}>Path {index + 1}</h4>
                  <dl>
                    <dt>Rule ID</dt>
                    <dd>{path.ruleId}</dd>
                    <dt>Explanation</dt>
                    <dd>{path.explanation}</dd>
                    <dt>Assertion</dt>
                    <dd>{path.assertion}</dd>
                    <dt>Confirmed vulnerability</dt>
                    <dd>
                      {path.isConfirmedVulnerability
                        ? 'Sí, confirmada'
                        : 'No confirmada'}
                    </dd>
                  </dl>
                </section>
              </li>
            ))}
          </ul>
        )}
      </section>
    </aside>
  );
}
