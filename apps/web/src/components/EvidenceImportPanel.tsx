import { useEffect, useRef, useState } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type { ScopePolicyDto, SnapshotDto } from '../contracts/snapshot';

interface Props {
  client: ApiClient;
  onImported: (snapshot: SnapshotDto) => void;
}

export function EvidenceImportPanel({ client, onImported }: Props) {
  const [jsonlFile, setJsonlFile] = useState<File | null>(null);
  const [scopeFile, setScopeFile] = useState<File | null>(null);
  const [presetId, setPresetId] = useState('');
  const [busy, setBusy] = useState(false);
  const [cancelable, setCancelable] = useState(false);
  const [status, setStatus] = useState('');
  const importController = useRef<AbortController | null>(null);

  useEffect(() => () => importController.current?.abort(), []);

  async function importFile() {
    if (!jsonlFile || !scopeFile || !client.importBbotJsonl || busy) return;
    if (jsonlFile.size > 2 * 1024 * 1024 || scopeFile.size > 64 * 1024) {
      setStatus(
        'El JSONL admite hasta 2 MiB y la política de scope hasta 64 KiB.',
      );
      return;
    }

    setBusy(true);
    setCancelable(true);
    setStatus('Validando e importando evidencia local…');
    const controller = new AbortController();
    importController.current = controller;
    try {
      const [jsonl, scopeText] = await Promise.all([
        jsonlFile.text(),
        scopeFile.text(),
      ]);
      const scope = parseScopePolicy(JSON.parse(scopeText));
      const snapshot = await client.importBbotJsonl(jsonl, scope, {
        signal: controller.signal,
        onPhase: (phase) => {
          setCancelable(
            !controller.signal.aborted &&
              (phase === 'started' || phase === 'event'),
          );
        },
        ...(presetId ? { presetId } : {}),
      });
      onImported(snapshot);
      setStatus('Importación terminada y guardada en el almacén local CHEF.');
    } catch (error) {
      setStatus(
        controller.signal.aborted &&
          error instanceof DOMException &&
          error.name === 'AbortError'
          ? 'Importación cancelada; no se guardó el snapshot.'
          : error instanceof Error
            ? `No se pudo importar: ${error.message}`
            : 'No se pudo importar el archivo BBOT.',
      );
    } finally {
      importController.current = null;
      setCancelable(false);
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="evidence-import-title">
      <h2 id="evidence-import-title">Importar evidencia JSONL</h2>
      <p>
        Importa un JSONL ya generado y una política CHEF explícita. La operación
        solo procesa datos locales; no inicia reconocimiento de red.
      </p>
      <label>
        Archivo de eventos (.jsonl o .ndjson)
        <input
          type="file"
          accept=".jsonl,.ndjson,application/x-ndjson,application/json"
          onChange={(event) =>
            setJsonlFile(event.currentTarget.files?.[0] ?? null)
          }
        />
      </label>
      <label>
        Política de scope CHEF (.json)
        <input
          type="file"
          accept="application/json,.json"
          onChange={(event) =>
            setScopeFile(event.currentTarget.files?.[0] ?? null)
          }
        />
      </label>
      <label>
        Perfil CHEF para esta importación
        <select
          value={presetId}
          disabled={busy}
          onChange={(event) => setPresetId(event.currentTarget.value)}
        >
          <option value="">Usar solo config.toml local</option>
          <option value="offline_import">Importación local</option>
          <option value="evidence_review">Revisión de evidencia</option>
          <option value="offline_hints">Importación con pistas offline</option>
        </select>
      </label>
      <p>
        El perfil se aplica solo a este trabajo y no modifica config.toml. Las
        pistas offline se derivan del archivo importado y no hacen consultas de
        red.
      </p>
      <button
        type="button"
        disabled={busy || !jsonlFile || !scopeFile}
        onClick={() => void importFile()}
      >
        {busy ? 'Importando…' : 'Importar JSONL'}
      </button>
      {busy && cancelable && (
        <button
          type="button"
          onClick={() => {
            setCancelable(false);
            importController.current?.abort();
          }}
        >
          Cancelar importación
        </button>
      )}
      <p role="status" aria-live="polite">
        {status}
      </p>
    </section>
  );
}

function parseScopePolicy(value: unknown): ScopePolicyDto {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('La política debe ser un objeto JSON.');
  }
  const policy = value as Record<string, unknown>;
  const keys = [
    'schemaVersion',
    'id',
    'allowedAddresses',
    'excludedAddresses',
    'allowedPorts',
    'allowedProtocols',
    'maxInputBytes',
    'maxObservations',
  ];
  if (
    Object.keys(policy).some((key) => !keys.includes(key)) ||
    keys.some((key) => !(key in policy)) ||
    policy.schemaVersion !== '1.0.0' ||
    typeof policy.id !== 'string' ||
    !stringArray(policy.allowedAddresses) ||
    !stringArray(policy.excludedAddresses) ||
    !numberArray(policy.allowedPorts) ||
    !Array.isArray(policy.allowedProtocols) ||
    !policy.allowedProtocols.every(
      (item) => item === 'tcp' || item === 'udp',
    ) ||
    !boundedInteger(policy.maxInputBytes, 1, 2 * 1024 * 1024) ||
    !boundedInteger(policy.maxObservations, 1, 10_000)
  ) {
    throw new Error('La política no cumple el contrato CHEF de scope 1.0.0.');
  }
  return policy as unknown as ScopePolicyDto;
}

function stringArray(value: unknown): value is string[] {
  return (
    Array.isArray(value) && value.every((item) => typeof item === 'string')
  );
}

function numberArray(value: unknown): value is number[] {
  return (
    Array.isArray(value) &&
    value.every((item) => boundedInteger(item, 1, 65_535))
  );
}

function boundedInteger(
  value: unknown,
  minimum: number,
  maximum: number,
): value is number {
  return (
    typeof value === 'number' &&
    Number.isInteger(value) &&
    value >= minimum &&
    value <= maximum
  );
}
