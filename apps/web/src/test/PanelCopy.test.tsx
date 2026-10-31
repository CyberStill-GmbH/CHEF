import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type { ApiClient } from '../api/ApiClient';
import { CensysIpImportPanel } from '../components/CensysIpImportPanel';
import { EvidenceImportPanel } from '../components/EvidenceImportPanel';
import { TargetSeedPanel } from '../components/TargetSeedPanel';
import { UrlScanImportPanel } from '../components/UrlScanImportPanel';

const client: ApiClient = { loadSnapshot: vi.fn() };

describe('copy de límites y uso local', () => {
  it('explica la clasificación local sin repetir detalles técnicos', () => {
    render(<TargetSeedPanel client={client} />);

    expect(
      screen.getByText(
        'Clasifica texto localmente: sin red ni disco, no añade al scope ni acredita propiedad.',
      ),
    ).toBeInTheDocument();
  });

  it('aclara que la respuesta Censys archivada no genera actividad de red', () => {
    render(<CensysIpImportPanel client={client} onImported={vi.fn()} />);

    expect(
      screen.getByText(
        'Sin Censys, DNS ni escaneo; son pistas históricas y su fecha de origen no se verifica.',
      ),
    ).toBeInTheDocument();
  });

  it('aclara el límite offline de urlscan', () => {
    render(<UrlScanImportPanel client={client} onImported={vi.fn()} />);

    expect(
      screen.getByText(
        'Sin consulta a urlscan ni DNS; son pistas históricas y su fecha de origen no se verifica.',
      ),
    ).toBeInTheDocument();
  });

  it('aclara que el JSONL no inicia reconocimiento', () => {
    render(<EvidenceImportPanel client={client} onImported={vi.fn()} />);

    expect(
      screen.getByText(
        'Procesa archivos locales; no genera tráfico ni inicia reconocimiento.',
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText('Perfil temporal CHEF (no cambia config.toml)'),
    ).toBeInTheDocument();
  });
});
