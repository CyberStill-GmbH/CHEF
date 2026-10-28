import { useCallback, useEffect, useState } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type {
  AsnReportDto,
  AsnReportEvidenceDto,
} from '../contracts/asn-report';
import type { ReconGraphDto } from '../contracts/recon-graph';

interface Props {
  client: ApiClient;
  runId: string;
  reconGraph: ReconGraphDto;
}

function evidenceMatches(
  graph: ReconGraphDto,
  eventId: string,
  eventType: 'IP_ADDRESS' | 'ASN',
  locator: string,
  observedAt: string,
) {
  return graph.evidence.find(
    (source) =>
      source.sourceEventId === eventId &&
      source.eventType === eventType &&
      source.locator === locator &&
      source.observedAt === observedAt,
  );
}

function reportEvidenceMatches(
  graph: ReconGraphDto,
  evidence: AsnReportEvidenceDto,
) {
  const ipEvidence = evidenceMatches(
    graph,
    evidence.ipEventId,
    'IP_ADDRESS',
    evidence.ipLocator,
    evidence.ipObservedAt,
  );
  const asnEvidence = evidenceMatches(
    graph,
    evidence.asnEventId,
    'ASN',
    evidence.asnLocator,
    evidence.asnObservedAt,
  );
  return (
    ipEvidence !== undefined &&
    asnEvidence !== undefined &&
    graph.assets.some(
      (asset) =>
        asset.kind === 'ip_address' &&
        asset.key === evidence.ip &&
        asset.evidenceIds.includes(ipEvidence.id),
    )
  );
}

export function AsnReportPanel({ client, runId, reconGraph }: Props) {
  const [report, setReport] = useState<AsnReportDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshCounter, setRefreshCounter] = useState(0);

  const loadReport = useCallback(
    async (signal: AbortSignal) => {
      if (client.loadAsnReport === undefined) return;
      setLoading(true);
      setError(null);
      setReport(null);
      try {
        const result = await client.loadAsnReport(runId, { signal });
        if (signal.aborted) return;
        if (result.runId !== runId) {
          throw new Error('El informe ASN corresponde a otra ejecución CHEF.');
        }
        const unresolvedReference = result.rows.some((row) =>
          row.evidence.some(
            (evidence) =>
              !row.ips.includes(evidence.ip) ||
              !reportEvidenceMatches(reconGraph, evidence),
          ),
        );
        if (unresolvedReference) {
          throw new Error(
            'El informe ASN contiene una referencia que no coincide con el grafo local; se ocultaron sus datos.',
          );
        }
        setReport(result);
      } catch (cause) {
        if (!signal.aborted) {
          setError(
            cause instanceof Error
              ? cause.message
              : 'No se pudo leer el informe ASN local.',
          );
        }
      } finally {
        if (!signal.aborted) setLoading(false);
      }
    },
    [client, reconGraph, runId],
  );

  useEffect(() => {
    const controller = new AbortController();
    void loadReport(controller.signal);
    return () => controller.abort();
  }, [loadReport, refreshCounter]);

  if (client.loadAsnReport === undefined) return null;

  return (
    <section aria-labelledby="asn-report-title" aria-busy={loading}>
      <h2 id="asn-report-title">Resumen ASN local</h2>
      <p>
        Agrupa únicamente relaciones directas entre eventos IP_ADDRESS y ASN
        importados. No consulta proveedores, no expande redes y no determina
        autorización de alcance.
      </p>
      <p>
        Nombres, descripciones, países y conteos de prefijos aparecen solo si
        esos metadatos estaban en los eventos guardados. CHEF no los completa ni
        muestra rangos deducidos.
      </p>
      <button
        type="button"
        onClick={() => setRefreshCounter((current) => current + 1)}
        disabled={loading}
      >
        Actualizar resumen local
      </button>
      {loading && <p role="status">Leyendo eventos ASN de esta ejecución…</p>}
      {error !== null && <p role="alert">{error}</p>}
      {report !== null &&
        (report.rows.length === 0 ? (
          <p role="status">
            No hay pares IP_ADDRESS–ASN que mostrar en esta importación.
          </p>
        ) : (
          <ul>
            {report.rows.map((row) => (
              <li key={row.asn}>
                <h3>AS{row.asn}</h3>
                <h4>Direcciones IP observadas</h4>
                <ul>
                  {row.ips.map((ip) => (
                    <li key={ip}>{ip}</li>
                  ))}
                </ul>
                <MetadataList label="Nombres reportados" values={row.names} />
                <MetadataList
                  label="Descripciones reportadas"
                  values={row.descriptions}
                />
                <MetadataList
                  label="Países reportados"
                  values={row.countries}
                />
                <MetadataList
                  label="Conteos de subredes reportados"
                  values={row.preservedSubnetCounts.map(String)}
                />
                <h4>Procedencia de los pares IP/ASN</h4>
                <ul>
                  {row.evidence.map((evidence) => (
                    <li
                      key={`${evidence.ipEventId}:${evidence.asnEventId}:${evidence.ipLocator}:${evidence.asnLocator}`}
                    >
                      <p>IP {evidence.ip}: evento IP_ADDRESS</p>
                      <p>
                        {evidence.ipObservedAt}; {evidence.ipLocator}; ID{' '}
                        {evidence.ipEventId}
                      </p>
                      <p>ASN: evento ASN</p>
                      <p>
                        {evidence.asnObservedAt}; {evidence.asnLocator}; ID{' '}
                        {evidence.asnEventId}
                        {evidence.asnModule === null
                          ? ''
                          : `; módulo informado ${evidence.asnModule}`}
                      </p>
                    </li>
                  ))}
                </ul>
              </li>
            ))}
          </ul>
        ))}
    </section>
  );
}

interface MetadataListProps {
  label: string;
  values: string[];
}

function MetadataList({ label, values }: MetadataListProps) {
  return (
    <>
      <h4>{label}</h4>
      {values.length === 0 ? (
        <p>No informado en la importación.</p>
      ) : (
        <ul>
          {values.map((value, index) => (
            <li key={`${value}:${index}`}>{value}</li>
          ))}
        </ul>
      )}
    </>
  );
}
