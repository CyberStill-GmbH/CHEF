import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import type { ApiClient } from '../api/ApiClient';
import { App } from '../App';
import { EvidencePanel } from '../components/EvidencePanel';
import { decodeSnapshot } from '../contracts/decodeSnapshot';
import type { AssetDto, SnapshotDto } from '../contracts/snapshot';
import fixture from '../../../../docs/contracts/examples/snapshot.json';

const fixtureSnapshot = decodeSnapshot(structuredClone(fixture));
const selectedAsset = fixtureSnapshot.assets[0]!;

function clientFor(snapshot: SnapshotDto): ApiClient {
  return { loadSnapshot: () => Promise.resolve(snapshot) };
}

function renderPanel(
  snapshot: SnapshotDto = fixtureSnapshot,
  asset: AssetDto = selectedAsset,
) {
  return render(<EvidencePanel snapshot={snapshot} asset={asset} />);
}

function namedSection(name: string): HTMLElement {
  const heading = screen.getByRole('heading', { name });
  const section = heading.closest('section');
  if (section === null) {
    throw new Error(`Expected section named ${name}.`);
  }
  return section;
}

describe('EvidencePanel', () => {
  it('appears after selecting an asset in App', async () => {
    const user = userEvent.setup();
    render(<App client={clientFor(fixtureSnapshot)} />);

    await user.click(await screen.findByText(selectedAsset.key));

    expect(
      screen.getByRole('complementary', { name: 'Asset seleccionado' }),
    ).toBeInTheDocument();
    expect(
      within(namedSection('Asset seleccionado')).getByText(selectedAsset.id),
    ).toBeInTheDocument();
  });

  it('shows the observations referenced by the selected asset', () => {
    renderPanel();
    const observations = within(namedSection('Observaciones relacionadas'));
    const expected = fixtureSnapshot.observations.find(
      ({ id }) => id === selectedAsset.observationIds[0],
    )!;

    expect(observations.getByText(expected.id)).toBeInTheDocument();
    expect(observations.getByText(expected.fact)).toBeInTheDocument();
    expect(observations.getByText(expected.source)).toBeInTheDocument();
    expect(observations.getByText(expected.observedAt)).toBeInTheDocument();
    expect(observations.getByText(expected.protocol)).toBeInTheDocument();
    expect(observations.getByText(String(expected.port))).toBeInTheDocument();
    expect(
      observations.getByText(String(expected.confidence)),
    ).toBeInTheDocument();
    expect(observations.getByText(expected.evidenceId)).toBeInTheDocument();
  });

  it('resolves and shows the evidence referenced by observations', () => {
    renderPanel();
    const evidence = within(namedSection('Evidence relacionada'));
    const expected = fixtureSnapshot.evidence[0]!;

    expect(evidence.getByText(expected.id)).toBeInTheDocument();
    expect(evidence.getByText(expected.parserVersion)).toBeInTheDocument();
    expect(evidence.getByText(expected.importedAt)).toBeInTheDocument();
  });

  it('shows the evidence digest and locator as text', () => {
    renderPanel();
    const evidence = within(namedSection('Evidence relacionada'));
    const expected = fixtureSnapshot.evidence[0]!;

    expect(evidence.getByText(expected.sha256)).toBeInTheDocument();
    expect(evidence.getByText(expected.locator)).toBeInTheDocument();
  });

  it('shows relationships connected to the asset', () => {
    renderPanel();
    const relationships = within(namedSection('Relaciones conectadas'));
    const expected = fixtureSnapshot.relationships.find(
      ({ from, to }) => from === selectedAsset.id || to === selectedAsset.id,
    )!;

    expect(relationships.getByText(expected.id)).toBeInTheDocument();
    expect(relationships.getByText(expected.kind)).toBeInTheDocument();
    expect(relationships.getByText(expected.assertion)).toBeInTheDocument();
    expect(relationships.getByText(expected.from)).toBeInTheDocument();
    expect(relationships.getByText(expected.to)).toBeInTheDocument();
  });

  it('shows path ruleId, explanation and assertion when present', () => {
    renderPanel();
    const paths = within(namedSection('Paths relacionados'));
    const expected = fixtureSnapshot.paths.find(({ assetIds }) =>
      assetIds.includes(selectedAsset.id),
    )!;

    expect(paths.getByText(expected.ruleId)).toBeInTheDocument();
    expect(paths.getByText(expected.explanation)).toBeInTheDocument();
    expect(paths.getByText(expected.assertion)).toBeInTheDocument();
  });

  it('states explicitly that vulnerability is not confirmed', () => {
    renderPanel();

    expect(
      within(namedSection('Paths relacionados')).getByText('No confirmada'),
    ).toBeInTheDocument();
  });

  it('handles an asset without observations', () => {
    renderPanel(fixtureSnapshot, {
      ...selectedAsset,
      observationIds: [],
    });

    expect(
      screen.getByText('Este asset no tiene observaciones referenciadas.'),
    ).toBeInTheDocument();
    expect(
      screen.getByText('No hay evidence resoluble para este asset.'),
    ).toBeInTheDocument();
  });

  it('shows a safe state when referenced evidence is missing', () => {
    const missingEvidenceId = `evidence:${'0'.repeat(64)}`;
    const observationId = selectedAsset.observationIds[0]!;
    const snapshot: SnapshotDto = {
      ...fixtureSnapshot,
      observations: fixtureSnapshot.observations.map((observation) =>
        observation.id === observationId
          ? { ...observation, evidenceId: missingEvidenceId }
          : observation,
      ),
    };

    expect(() => renderPanel(snapshot)).not.toThrow();
    expect(
      screen.getByText(
        `La evidence referenciada no está disponible: ${missingEvidenceId}`,
      ),
    ).toBeInTheDocument();
  });

  it('renders hostile imported text without activating HTML', () => {
    const hostileText = '<img src=x onerror="alert(1)">';
    const snapshot: SnapshotDto = {
      ...fixtureSnapshot,
      evidence: fixtureSnapshot.evidence.map((evidence, index) =>
        index === 0 ? { ...evidence, locator: hostileText } : evidence,
      ),
    };
    const view = renderPanel(snapshot, {
      ...selectedAsset,
      key: hostileText,
    });

    expect(screen.getAllByText(hostileText)).toHaveLength(2);
    expect(view.container.querySelector('img')).toBeNull();
  });

  it('does not create links from imported locators or keys', () => {
    const importedUrl = 'https://attacker.invalid/evidence';
    const snapshot: SnapshotDto = {
      ...fixtureSnapshot,
      evidence: fixtureSnapshot.evidence.map((evidence, index) =>
        index === 0 ? { ...evidence, locator: importedUrl } : evidence,
      ),
    };
    renderPanel(snapshot, { ...selectedAsset, key: importedUrl });

    expect(screen.getAllByText(importedUrl)).toHaveLength(2);
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
  });

  it('updates the panel when the selected asset changes', async () => {
    const user = userEvent.setup();
    const nextAsset = fixtureSnapshot.assets[1]!;
    render(<App client={clientFor(fixtureSnapshot)} />);

    await user.click(await screen.findByText(selectedAsset.key));
    let assetDetails = within(namedSection('Asset seleccionado'));
    expect(assetDetails.getByText(selectedAsset.id)).toBeInTheDocument();

    await user.click(screen.getByText(nextAsset.key));
    assetDetails = within(namedSection('Asset seleccionado'));
    expect(assetDetails.getByText(nextAsset.id)).toBeInTheDocument();
    expect(assetDetails.queryByText(selectedAsset.id)).not.toBeInTheDocument();
  });
});
