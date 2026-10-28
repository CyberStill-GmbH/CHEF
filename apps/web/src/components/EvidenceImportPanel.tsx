import { useState } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type { ScopePolicyDto, SnapshotDto } from '../contracts/snapshot';

interface Props {
  client: ApiClient;
  onImported: (snapshot: SnapshotDto) => void;
}

export function EvidenceImportPanel({ client, onImported }: Props) {
  const [jsonlFile, setJsonlFile] = useState<File | null>(null);
  const [scopeFile, setScopeFile] = useState<File | null>(null);
  const [dnsWordlistFile, setDnsWordlistFile] = useState<File | null>(null);
  const [presetId, setPresetId] = useState('');
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState('');

  async function importFile() {
    if (!jsonlFile || !scopeFile || !client.importBbotJsonl || busy) return;
    if (
      jsonlFile.size > 2 * 1024 * 1024 ||
      scopeFile.size > 64 * 1024 ||
      (dnsWordlistFile && dnsWordlistFile.size > 2 * 1024 * 1024)
    ) {
      setStatus(
        'El JSONL y la lista DNS admiten hasta 2 MiB cada uno; la política de scope, hasta 64 KiB.',
      );
      return;
    }
    if (dnsWordlistFile && presetId !== 'offline_hints') {
      setStatus(
        'Para usar una lista DNS, selecciona el perfil “Importación con pistas offline”.',
      );
      return;
    }

    setBusy(true);
    setStatus('Validando e importando evidencia local…');
    try {
      const [jsonl, scopeText, dnsWordlist] = await Promise.all([
        jsonlFile.text(),
        scopeFile.text(),
        dnsWordlistFile?.text(),
      ]);
      const scope = parseScopePolicy(JSON.parse(scopeText));
      const snapshot = await client.importBbotJsonl(jsonl, scope, {
        ...(presetId ? { presetId } : {}),
        ...(dnsWordlist !== undefined ? { dnsWordlist } : {}),
      });
      onImported(snapshot);
      setStatus('Importación terminada y guardada en el almacén local CHEF.');
    } catch (error) {
      setStatus(
        error instanceof Error
          ? `No se pudo importar: ${error.message}`
          : 'No se pudo importar el archivo BBOT.',
      );
    } finally {
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
        Lista DNS local opcional (.txt, hasta 2 MiB)
        <input
          type="file"
          accept="text/plain,.txt"
          disabled={busy}
          onChange={(event) =>
            setDnsWordlistFile(event.currentTarget.files?.[0] ?? null)
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
