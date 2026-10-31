import { useState } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type { UrlScanImportSummaryDto } from '../contracts/urlscan-import';
import type { ReconGraphDto } from '../contracts/recon-graph';
import type { ScanLogPhase } from '../contracts/scan-log';

interface Props {
  client: ApiClient;
  onImported: (summary: UrlScanImportSummaryDto, graph: ReconGraphDto) => void;
}

const MAX_RESPONSE_BYTES = 2 * 1024 * 1024;

export function UrlScanImportPanel({ client, onImported }: Props) {
  const [queryDomain, setQueryDomain] = useState('');
  const [responseFile, setResponseFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState('');

  async function importResponse() {
    if (
      !client.importUrlScanResponse ||
      !client.loadUrlScanReconGraph ||
      !responseFile ||
      busy
    )
      return;
    if (!queryDomain.trim()) {
      setStatus('Escribe el dominio consultado en el archivo archivado.');
      return;
    }
    if (responseFile.size > MAX_RESPONSE_BYTES) {
      setStatus('La respuesta JSON admite hasta 2 MiB.');
      return;
    }
    setBusy(true);
    setStatus('Validando el archivo local…');
    try {
      const summary = await client.importUrlScanResponse(
        queryDomain.trim(),
        await responseFile.text(),
        {
          onPhase: (phase) => {
            const message = phaseStatus(phase);
            if (message) setStatus(message);
          },
        },
      );
      const graph = await client.loadUrlScanReconGraph(summary.importId);
      onImported(summary, graph);
      setStatus('Respuesta importada y guardada en SurrealDB local.');
    } catch (error) {
      setStatus(
        error instanceof Error
          ? `No se pudo importar la respuesta: ${error.message}`
          : 'No se pudo importar la respuesta urlscan.',
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="urlscan-import-title">
      <h2 id="urlscan-import-title">Importar resultados urlscan archivados</h2>
      <p>
        Sin consulta a urlscan ni DNS; son pistas históricas y su fecha de
        origen no se verifica.
      </p>
      <label>
        Dominio consultado
        <input
          autoComplete="off"
          value={queryDomain}
          disabled={busy}
          onChange={(event) => setQueryDomain(event.currentTarget.value)}
          placeholder="example.com"
        />
      </label>
      <label>
        Respuesta archivada (.json)
        <input
          type="file"
          accept="application/json,.json"
          disabled={busy}
          onChange={(event) =>
            setResponseFile(event.currentTarget.files?.[0] ?? null)
          }
        />
      </label>
      <button
        type="button"
        disabled={busy || responseFile === null || !queryDomain.trim()}
        onClick={() => void importResponse()}
      >
        {busy ? 'Importando…' : 'Importar respuesta offline'}
      </button>
      <p role="status" aria-live="polite">
        {status}
      </p>
    </section>
  );
}

function phaseStatus(phase: ScanLogPhase): string | undefined {
  const messages: Partial<Record<ScanLogPhase, string>> = {
    started: 'Importación offline iniciada…',
    validated: 'Respuesta normalizada; guardando evidencia…',
    stored: 'Evidencia guardada en el almacén local…',
    completed: 'Importación offline completada.',
    failed: 'La respuesta no pudo validarse o guardarse.',
  };
  return messages[phase];
}
