import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import type { ApiClient } from '../api/ApiClient';
import { UrlScanImportPanel } from '../components/UrlScanImportPanel';
import type { UrlScanImportSummaryDto } from '../contracts/urlscan-import';
import type { ReconGraphDto } from '../contracts/recon-graph';

const summary: UrlScanImportSummaryDto = {
  schemaVersion: '1.0.0',
  importId: `urlscan-import:${'a'.repeat(64)}`,
  queryDomain: 'example.com',
  responseSha256: 'b'.repeat(64),
  importedAt: '2026-10-31T00:00:00.000Z',
  eventCount: 3,
  assetCount: 2,
  relationshipCount: 1,
  unprojectedEventTypes: {},
};

const graph = {} as ReconGraphDto;

function makeClient() {
  const importUrlScanResponse = vi.fn().mockResolvedValue(summary);
  const loadUrlScanReconGraph = vi.fn().mockResolvedValue(graph);
  const client: ApiClient = {
    loadSnapshot: vi.fn(),
    importUrlScanResponse,
    loadUrlScanReconGraph,
  };
  return { client, importUrlScanResponse, loadUrlScanReconGraph };
}

describe('UrlScanImportPanel', () => {
  it('imports an archived JSON locally and returns its graph', async () => {
    const user = userEvent.setup();
    const { client, importUrlScanResponse, loadUrlScanReconGraph } =
      makeClient();
    const onImported = vi.fn();
    render(<UrlScanImportPanel client={client} onImported={onImported} />);

    await user.type(
      screen.getByLabelText('Dominio consultado'),
      ' example.com ',
    );
    await user.upload(
      screen.getByLabelText('Respuesta archivada (.json)'),
      new File(['{"results":[]}'], 'response.json', {
        type: 'application/json',
      }),
    );
    await user.click(
      screen.getByRole('button', { name: 'Importar respuesta offline' }),
    );

    await waitFor(() =>
      expect(onImported).toHaveBeenCalledWith(summary, graph),
    );
    expect(importUrlScanResponse).toHaveBeenCalledWith(
      'example.com',
      '{"results":[]}',
      expect.objectContaining({ onPhase: expect.any(Function) }),
    );
    expect(loadUrlScanReconGraph).toHaveBeenCalledWith(summary.importId);
    expect(screen.getByRole('status')).toHaveTextContent(
      'Respuesta importada y guardada en SurrealDB local.',
    );
  });

  it('rejects a response larger than 2 MiB without calling the client', async () => {
    const user = userEvent.setup();
    const { client, importUrlScanResponse } = makeClient();
    render(<UrlScanImportPanel client={client} onImported={vi.fn()} />);
    await user.type(screen.getByLabelText('Dominio consultado'), 'example.com');

    const oversized = new File(['x'], 'large.json', {
      type: 'application/json',
    });
    Object.defineProperty(oversized, 'size', { value: 2 * 1024 * 1024 + 1 });
    await user.upload(
      screen.getByLabelText('Respuesta archivada (.json)'),
      oversized,
    );
    await user.click(
      screen.getByRole('button', { name: 'Importar respuesta offline' }),
    );

    expect(screen.getByRole('status')).toHaveTextContent(
      'La respuesta JSON admite hasta 2 MiB.',
    );
    expect(importUrlScanResponse).not.toHaveBeenCalled();
  });

  it('keeps importing disabled until a query domain is provided', async () => {
    const user = userEvent.setup();
    const { client, importUrlScanResponse } = makeClient();
    render(<UrlScanImportPanel client={client} onImported={vi.fn()} />);
    await user.upload(
      screen.getByLabelText('Respuesta archivada (.json)'),
      new File(['{}'], 'response.json', { type: 'application/json' }),
    );

    expect(
      screen.getByRole('button', { name: 'Importar respuesta offline' }),
    ).toBeDisabled();
    expect(importUrlScanResponse).not.toHaveBeenCalled();
  });
});
