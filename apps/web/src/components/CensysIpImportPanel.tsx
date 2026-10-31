import { useState } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type { CensysIpImportSummaryDto } from '../contracts/censys-ip-import';
import type { ReconGraphDto } from '../contracts/recon-graph';
import type { ScanLogPhase } from '../contracts/scan-log';

interface Props {
  client: ApiClient;
  onImported: (summary: CensysIpImportSummaryDto, graph: ReconGraphDto) => void;
}

const MAX_RESPONSE_BYTES = 2 * 1024 * 1024;

export function CensysIpImportPanel({ client, onImported }: Props) {
  const [targetIp, setTargetIp] = useState('');
  const [responseFile, setResponseFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState('');

  async function importResponse() {
    if (
      !client.importCensysIpResponse ||
      !client.loadCensysIpReconGraph ||
      responseFile === null ||
      busy
    ) {
      return;
    }
    if (!targetIp.trim()) {
      setStatus('Escribe la IP consultada en la respuesta archivada.');
      return;
    }
    if (responseFile.size > MAX_RESPONSE_BYTES) {
      setStatus('La respuesta JSON admite hasta 2 MiB.');
      return;
    }

    setBusy(true);
    setStatus('Validando el archivo local…');
    try {
      const responseJson = await responseFile.text();
      const summary = await client.importCensysIpResponse(
        targetIp.trim(),
        responseJson,
        {
          onPhase: (phase) => {
            const message = phaseStatus(phase);
            if (message) setStatus(message);
          },
        },
      );
      const graph = await client.loadCensysIpReconGraph(summary.importId);
      onImported(summary, graph);
      setStatus('Respuesta importada y guardada en SurrealDB local.');
    } catch (error) {
      setStatus(
        error instanceof Error
          ? `No se pudo importar la respuesta: ${error.message}`
          : 'No se pudo importar la respuesta Censys.',
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="censys-import-title">
      <h2 id="censys-import-title">Importar respuesta Censys archivada</h2>
      <p>
        Procesa un JSON local de Hosts por IP. CHEF no consulta Censys ni hace
        DNS o escaneo activo; los resultados se muestran como pistas históricas.
      </p>
      <label>
        IP consultada
        <input
          autoComplete="off"
          inputMode="decimal"
          value={targetIp}
          disabled={busy}
          onChange={(event) => setTargetIp(event.currentTarget.value)}
          placeholder="198.51.100.10"
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
      <p>
        El timestamp registra cuándo se importó el archivo; la respuesta no
        aporta una fecha fuente fiable. La UI conserva el SHA-256 y el linaje.
      </p>
      <button
        type="button"
        disabled={busy || responseFile === null || !targetIp.trim()}
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
