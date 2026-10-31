import { useState, type ReactNode } from 'react';

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

  async function exportWebEvidenceReport() {
    if (client.exportWebEvidenceReport === undefined || busy) return;
    setBusy(true);
    setStatus('Preparando informe local de evidencia web…');
    try {
      const exported = await client.exportWebEvidenceReport(snapshot.run.id);
      const blob = new Blob([exported.html], { type: exported.contentType });
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
        `Informe descargado (${exported.endpointCount} endpoints y ${exported.candidateFindingCount} candidatos relacionados).`,
      );
    } catch {
      setStatus('No se pudo exportar el informe web CHEF.');
    } finally {
      setBusy(false);
    }
  }

  async function exportObservedEmails() {
    if (client.exportObservedEmails === undefined || busy) return;
    setBusy(true);
    setStatus('Preparando exportación local de correos observados…');
    try {
      const exported = await client.exportObservedEmails(snapshot.run.id);
      const blob = new Blob([`${JSON.stringify(exported, null, 2)}\n`], {
        type: exported.contentType,
      });
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
        `Correos observados exportados (${exported.emailCount} direcciones únicas).`,
      );
    } catch {
      setStatus('No se pudieron exportar los correos observados.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="export-panel" aria-labelledby="snapshot-export-title">
      <h2 id="snapshot-export-title">Exportación local</h2>
      <div className="export-grid">
        <ExportOption
          title="Snapshot"
          description="Datos completos del snapshot CHEF en JSON."
        >
          <button
            type="button"
            disabled={busy || client.exportSnapshot === undefined}
            onClick={() => void exportSnapshot()}
          >
            {busy ? 'Preparando…' : 'Descargar snapshot JSON'}
          </button>
        </ExportOption>
        <ExportOption
          title="Dominios observados"
          description="Solo nombres con eventos DNS_NAME directos; excluye candidatos y nombres sin resolver."
        >
          <button
            type="button"
            disabled={
              busy ||
              client.exportObservedDomains === undefined ||
              !hasReconGraph
            }
            onClick={() => void exportObservedDomains()}
          >
            {busy ? 'Preparando…' : 'Descargar dominios observados JSON'}
          </button>
        </ExportOption>
        <ExportOption
          title="Evidencia"
          description="CSV con IDs y linaje. Omite payloads originales y no afirma alcance."
        >
          <button
            type="button"
            disabled={
              busy || client.exportEvidenceCsv === undefined || !hasReconGraph
            }
            onClick={() => void exportEvidenceCsv()}
          >
            {busy ? 'Preparando…' : 'Descargar evidencia CHEF CSV'}
          </button>
        </ExportOption>
        <ExportOption
          title="Puertos observados"
          description="XML CHEF con evidencia directa de puertos; no es un escaneo actual ni cumple el DTD oficial de Nmap."
        >
          <button
            type="button"
            disabled={
              busy || client.exportNmapEvidence === undefined || !hasReconGraph
            }
            onClick={() => void exportNmapEvidence()}
          >
            {busy ? 'Preparando…' : 'Descargar XML CHEF (vocabulario Nmap)'}
          </button>
        </ExportOption>
        <ExportOption
          title="Inventario"
          description="Hosts, IP, puertos, endpoints, tecnologías y hallazgos candidatos sustentados por relaciones."
        >
          <button
            type="button"
            disabled={
              busy ||
              client.exportAssetInventory === undefined ||
              !hasReconGraph
            }
            onClick={() => void exportAssetInventory()}
          >
            {busy ? 'Preparando…' : 'Descargar inventario de activos CSV'}
          </button>
        </ExportOption>
        <ExportOption
          title="Informe web"
          description="Informe HTML estático y local; no carga recursos remotos ni clasifica riesgo."
        >
          <button
            type="button"
            disabled={
              busy ||
              client.exportWebEvidenceReport === undefined ||
              !hasReconGraph
            }
            onClick={() => void exportWebEvidenceReport()}
          >
            {busy ? 'Preparando…' : 'Descargar informe web HTML'}
          </button>
        </ExportOption>
      </div>
      <details className="sensitive-export">
        <summary>
          Exportar correos observados · contiene datos personales
        </summary>
        <p>
          Descarga en JSON las direcciones EMAIL_ADDRESS ya presentes en esta
          importación. No envía datos ni deriva correos de nombres de usuario.
        </p>
        <button
          type="button"
          disabled={
            busy || client.exportObservedEmails === undefined || !hasReconGraph
          }
          onClick={() => void exportObservedEmails()}
        >
          {busy ? 'Preparando…' : 'Confirmar y descargar correos JSON'}
        </button>
      </details>
      <p role="status" aria-live="polite">
        {status}
      </p>
    </section>
  );
}

function ExportOption({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <article className="export-option">
      <div>
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
      {children}
    </article>
  );
}

function safeFilePart(value: string): string {
  return value.replace(/[^a-z0-9._-]/gi, '_').slice(0, 96) || 'snapshot';
}
