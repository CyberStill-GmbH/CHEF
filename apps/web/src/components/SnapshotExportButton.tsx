import { useState } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type { SnapshotDto } from '../contracts/snapshot';

interface SnapshotExportButtonProps {
  client: ApiClient;
  snapshot: SnapshotDto;
  hasReconGraph: boolean;
}

export function SnapshotExportButton({
  client,
  snapshot,
  hasReconGraph,
}: SnapshotExportButtonProps) {
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);

  async function exportSnapshot() {
    if (client.exportSnapshot === undefined || busy) return;
    setBusy(true);
    setStatus('Preparando la descarga…');
    try {
      const exported = await client.exportSnapshot(snapshot.run.id);
      const blob = new Blob([`${JSON.stringify(exported, null, 2)}\n`], {
        type: 'application/json;charset=utf-8',
      });
      const objectUrl = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = objectUrl;
      anchor.download = `chef-${safeFilePart(snapshot.run.id)}.json`;
      anchor.hidden = true;
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
      setStatus('Snapshot descargado en formato CHEF 1.0/1.1.');
    } catch {
      setStatus('No se pudo exportar el snapshot. Inténtalo de nuevo.');
    } finally {
      setBusy(false);
    }
  }

  async function exportObservedDomains() {
    if (client.exportObservedDomains === undefined || busy) return;
    setBusy(true);
    setStatus('Preparando dominios observados con su evidencia…');
    try {
      const exported = await client.exportObservedDomains(snapshot.run.id);
      const blob = new Blob([`${JSON.stringify(exported, null, 2)}\n`], {
        type: 'application/json;charset=utf-8',
      });
      const objectUrl = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = objectUrl;
      anchor.download = `chef-dominios-observados-${safeFilePart(snapshot.run.id)}.json`;
      anchor.hidden = true;
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
      setStatus('Dominios observados descargados con evidencia y procedencia.');
    } catch {
      setStatus('No se pudieron exportar los dominios observados.');
    } finally {
      setBusy(false);
    }
  }

  async function exportEvidenceCsv() {
    if (client.exportEvidenceCsv === undefined || busy) return;
    setBusy(true);
    setStatus('Preparando CSV de evidencia CHEF…');
    try {
      const exported = await client.exportEvidenceCsv(snapshot.run.id);
      const blob = new Blob([exported.csv], { type: exported.contentType });
      const objectUrl = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = objectUrl;
      anchor.download = exported.fileName;
      anchor.hidden = true;
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
      setStatus(`CSV de evidencia descargado (${exported.rowCount} filas).`);
    } catch {
      setStatus('No se pudo exportar el CSV de evidencia.');
    } finally {
      setBusy(false);
    }
  }

  async function exportNmapEvidence() {
    if (client.exportNmapEvidence === undefined || busy) return;
    setBusy(true);
    setStatus('Preparando XML de puertos observados…');
    try {
      const exported = await client.exportNmapEvidence(snapshot.run.id);
      const blob = new Blob([exported.xml], { type: exported.contentType });
      const objectUrl = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = objectUrl;
      anchor.download = exported.fileName;
      anchor.hidden = true;
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
      setStatus(
        `XML descargado: ${exported.hostCount} hosts y ${exported.portCount} puertos observados.`,
      );
    } catch {
      setStatus('No se pudo exportar el XML CHEF.');
    } finally {
      setBusy(false);
    }
  }

  async function exportAssetInventory() {
    if (client.exportAssetInventory === undefined || busy) return;
    setBusy(true);
    setStatus('Preparando inventario de activos CHEF…');
    try {
      const exported = await client.exportAssetInventory(snapshot.run.id);
      const blob = new Blob([exported.csv], { type: exported.contentType });
      const objectUrl = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = objectUrl;
      anchor.download = exported.fileName;
      anchor.hidden = true;
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
      setStatus(`Inventario descargado (${exported.rowCount} hosts).`);
    } catch {
      setStatus('No se pudo exportar el inventario CHEF.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="snapshot-export-title">
      <h2 id="snapshot-export-title">Exportación local</h2>
      <button
        type="button"
        disabled={busy || client.exportSnapshot === undefined}
        onClick={() => void exportSnapshot()}
      >
        {busy ? 'Preparando…' : 'Descargar snapshot JSON'}
      </button>
      <button
        type="button"
        disabled={
          busy || client.exportObservedDomains === undefined || !hasReconGraph
        }
        onClick={() => void exportObservedDomains()}
      >
        {busy ? 'Preparando…' : 'Descargar dominios observados JSON'}
      </button>
      <p>
        La lista incluye nombres sustentados por eventos DNS_NAME directos.
        Excluye candidatos y nombres sin resolver; no afirma alcance ni
        propiedad.
      </p>
      <button
        type="button"
        disabled={
          busy || client.exportEvidenceCsv === undefined || !hasReconGraph
        }
        onClick={() => void exportEvidenceCsv()}
      >
        {busy ? 'Preparando…' : 'Descargar evidencia CHEF CSV'}
      </button>
      <p>
        El CSV mantiene IDs y linaje, conserva evidencia sin activo adaptado y
        no incluye payloads originales ni nombres de módulo no verificados;
        conserva claves CHEF proyectadas y no afirma alcance.
      </p>
      <button
        type="button"
        disabled={
          busy || client.exportNmapEvidence === undefined || !hasReconGraph
        }
        onClick={() => void exportNmapEvidence()}
      >
        {busy ? 'Preparando…' : 'Descargar XML CHEF (vocabulario Nmap)'}
      </button>
      <p>
        Incluye solo puertos TCP/UDP con evidencia directa y relación observada
        a una IP. No es un resultado de escaneo actual, no afirma alcance y no
        cumple el DTD oficial de Nmap ni garantiza interoperabilidad con sus
        herramientas.
      </p>
      <button
        type="button"
        disabled={
          busy || client.exportAssetInventory === undefined || !hasReconGraph
        }
        onClick={() => void exportAssetInventory()}
      >
        {busy ? 'Preparando…' : 'Descargar inventario de activos CSV'}
      </button>
      <p>
        Resume hosts e IP, puertos abiertos, endpoints, tecnologías e IDs de
        hallazgos candidatos solo cuando el grafo conserva la relación y
        evidencia. No exporta descripciones ni clasifica riesgo o alcance.
      </p>
      <p role="status" aria-live="polite">
        {status}
      </p>
    </section>
  );
}

function safeFilePart(value: string): string {
  return value.replace(/[^a-z0-9._-]/gi, '_').slice(0, 96) || 'snapshot';
}
