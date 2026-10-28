import { useEffect, useRef, useState, type FormEvent } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type { DomainAffinityReportDto } from '../contracts/domain-affinity';
import type { ReconGraphDto } from '../contracts/recon-graph';

interface Props {
  client: ApiClient;
  runId: string;
  reconGraph: ReconGraphDto;
}

export function DomainAffinityPanel({ client, runId, reconGraph }: Props) {
  const [input, setInput] = useState('');
  const [roots, setRoots] = useState<string[]>([]);
  const [report, setReport] = useState<DomainAffinityReportDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const controllerRef = useRef<AbortController | null>(null);

  useEffect(
    () => () => {
      controllerRef.current?.abort();
    },
    [],
  );

  function addRoot(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const root = input.trim();
    if (!root) return;
    if (roots.some((item) => item.toLowerCase() === root.toLowerCase())) {
      setError('Esa raíz ya está en la lista.');
      return;
    }
    setRoots((current) => [...current, root]);
    setInput('');
    setReport(null);
    setError(null);
  }

  async function analyze() {
    if (client.analyzeDomainAffinity === undefined || roots.length === 0)
      return;
    controllerRef.current?.abort();
    const controller = new AbortController();
    controllerRef.current = controller;
    setLoading(true);
    setReport(null);
    setError(null);
    try {
      const result = await client.analyzeDomainAffinity(runId, roots, {
        signal: controller.signal,
      });
      if (result.runId !== runId) {
        throw new Error('El informe corresponde a otra ejecución CHEF.');
      }
      const unresolvedReference = result.candidates.some((candidate) =>
        candidate.evidence.some(
          (reference) =>
            !reconGraph.evidence.some(
              (source) =>
                source.sourceEventId === reference.eventId &&
                source.eventType === reference.eventType &&
                source.locator === reference.locator &&
                source.observedAt === reference.observedAt,
            ),
        ),
      );
      if (unresolvedReference) {
        throw new Error(
          'El informe contiene evidencia que no pertenece al grafo de esta ejecución; se ocultó el resultado.',
        );
      }
      if (!controller.signal.aborted) setReport(result);
    } catch (cause) {
      if (!controller.signal.aborted) {
        setError(
          cause instanceof Error
            ? cause.message
            : 'No se pudo analizar la afinidad local.',
        );
      }
    } finally {
      if (!controller.signal.aborted) setLoading(false);
    }
  }

  if (client.analyzeDomainAffinity === undefined) return null;

  return (
    <section aria-labelledby="domain-affinity-title">
      <h2 id="domain-affinity-title">Afinidad nominal de dominios</h2>
      <p>
        Análisis local de eventos DNS_NAME importados. Las raíces que añadas son
        criterios para agrupar evidencia de este grafo; no autorizan consultas,
        no prueban propiedad y no cambian el alcance.
      </p>
      <form onSubmit={addRoot}>
        <label htmlFor="domain-affinity-root">Raíz DNS registrable</label>
        <input
          id="domain-affinity-root"
          type="text"
          autoComplete="off"
          value={input}
          onChange={(event) => setInput(event.target.value)}
          placeholder="example.org"
          aria-describedby="domain-affinity-root-help"
          disabled={loading}
        />
        <p id="domain-affinity-root-help">
          Añade explícitamente de 1 a 64 raíces. El núcleo valida el formato.
        </p>
        <button
          type="submit"
          disabled={loading || !input.trim() || roots.length >= 64}
        >
          Añadir raíz
        </button>
      </form>
      {roots.length > 0 && (
        <div aria-label="Raíces elegidas para el análisis">
          <h3>Raíces elegidas</h3>
          <ul>
            {roots.map((root) => (
              <li key={root}>
                {root}{' '}
                <button
                  type="button"
                  aria-label={`Quitar raíz ${root}`}
                  onClick={() => {
                    setRoots((current) =>
                      current.filter((item) => item !== root),
                    );
                    setReport(null);
                  }}
                  disabled={loading}
                >
                  Quitar
                </button>
              </li>
            ))}
          </ul>
          <button
            type="button"
            onClick={() => void analyze()}
            disabled={loading}
          >
            Analizar evidencia local
          </button>
        </div>
      )}
      {loading && <p role="status">Analizando el grafo local…</p>}
      {error !== null && <p role="alert">{error}</p>}
      {report !== null &&
        (report.candidates.length === 0 ? (
          <p role="status">No se encontraron candidatos con estas raíces.</p>
        ) : (
          <div>
            <h3>Candidatos para revisión humana</h3>
            <ul>
              {report.candidates.map((candidate) => (
                <li key={candidate.id}>
                  <h4>{candidate.candidateDomain}</h4>
                  <p>
                    Raíces de contexto: {candidate.analysisRoots.join(', ')}
                  </p>
                  <p>{candidate.reason}</p>
                  <p>
                    Prioridad heurística: {candidate.score} (no es puntaje de
                    riesgo ni evidencia de propiedad). Regla {candidate.ruleId}/
                    {candidate.ruleVersion}.
                  </p>
                  <h5>Evidencia fuente</h5>
                  <ul>
                    {candidate.evidence.map((evidence) => (
                      <li key={`${evidence.eventId}:${evidence.locator}`}>
                        {evidence.eventType}; evento {evidence.eventId};{' '}
                        {evidence.observedAt}; localizador {evidence.locator};
                        distancia de origen {evidence.scopeDistance}; etiqueta
                        affiliate: {evidence.affiliateTagged ? 'sí' : 'no'}.
                        Estos datos describen el evento fuente, no autorizan
                        consultas ni alcance.
                      </li>
                    ))}
                  </ul>
                </li>
              ))}
            </ul>
          </div>
        ))}
    </section>
  );
}
