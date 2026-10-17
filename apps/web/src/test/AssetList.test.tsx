import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import type { ApiClient } from '../api/ApiClient';
import { App } from '../App';
import { AssetList } from '../components/AssetList';
import { decodeSnapshot } from '../contracts/decodeSnapshot';
import type { AssetDto, SnapshotDto } from '../contracts/snapshot';
import fixture from '../../../../docs/contracts/examples/snapshot.json';

const fixtureSnapshot = decodeSnapshot(structuredClone(fixture));

function clientFor(snapshot: SnapshotDto): ApiClient {
  return { loadSnapshot: () => Promise.resolve(snapshot) };
}

function buttonFor(asset: AssetDto): HTMLButtonElement {
  const heading = screen.getByRole('heading', { name: 'Assets disponibles' });
  const list = heading.closest('section');
  if (list === null) {
    throw new Error('Expected an accessible asset list section.');
  }
  const button = within(list).getByText(asset.key).closest('button');
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error('Expected asset key inside a button.');
  }
  return button;
}

function withoutAsset(assetId: string): SnapshotDto {
  const candidate = structuredClone(fixture);
  candidate.assets = candidate.assets.filter(({ id }) => id !== assetId);
  candidate.relationships = candidate.relationships.filter(
    ({ from, to }) => from !== assetId && to !== assetId,
  );
  const remainingRelationshipIds = new Set(
    candidate.relationships.map(({ id }) => id),
  );
  candidate.paths = candidate.paths.filter(
    ({ assetIds, relationshipIds }) =>
      !assetIds.includes(assetId) &&
      relationshipIds.every((id) => remainingRelationshipIds.has(id)),
  );
  return decodeSnapshot(candidate);
}

describe('AssetList', () => {
  it('renders every asset from the canonical fixture', () => {
    render(
      <AssetList
        assets={fixtureSnapshot.assets}
        selectedAssetId={null}
        onSelect={vi.fn()}
      />,
    );

    expect(screen.getAllByRole('button')).toHaveLength(
      fixtureSnapshot.assets.length,
    );
    for (const asset of fixtureSnapshot.assets) {
      expect(screen.getByText(asset.key)).toBeInTheDocument();
      expect(screen.getAllByText(asset.kind).length).toBeGreaterThan(0);
      expect(screen.getAllByText(asset.firstSeen).length).toBeGreaterThan(0);
      expect(screen.getAllByText(asset.lastSeen).length).toBeGreaterThan(0);
    }
  });

  it('selects an asset by click', async () => {
    const user = userEvent.setup();
    const asset = fixtureSnapshot.assets[0]!;
    render(<App client={clientFor(fixtureSnapshot)} />);

    await user.click(await screen.findByText(asset.key));

    expect(
      screen.getByRole('heading', { name: 'Asset seleccionado' }),
    ).toBeInTheDocument();
  });

  it('selects an asset using the native keyboard behavior', async () => {
    const user = userEvent.setup();
    const asset = fixtureSnapshot.assets[0]!;
    render(<App client={clientFor(fixtureSnapshot)} />);
    await screen.findByText(asset.key);
    const button = buttonFor(asset);
    button.focus();

    await user.keyboard('{Enter}');

    expect(button).toHaveAttribute('aria-pressed', 'true');
  });

  it('marks the selected asset accessibly', async () => {
    const user = userEvent.setup();
    const asset = fixtureSnapshot.assets[1]!;
    render(<App client={clientFor(fixtureSnapshot)} />);
    await screen.findByText(asset.key);
    const button = buttonFor(asset);

    await user.click(button);

    expect(button).toHaveAttribute('aria-pressed', 'true');
    for (const other of fixtureSnapshot.assets.filter(
      ({ id }) => id !== asset.id,
    )) {
      expect(buttonFor(other)).toHaveAttribute('aria-pressed', 'false');
    }
  });

  it('shows only the minimal selected asset fields', async () => {
    const user = userEvent.setup();
    const asset = fixtureSnapshot.assets[0]!;
    render(<App client={clientFor(fixtureSnapshot)} />);

    await user.click(await screen.findByText(asset.key));
    const heading = screen.getByRole('heading', { name: 'Asset seleccionado' });
    const detail = heading.closest('section');
    expect(detail).not.toBeNull();
    const content = within(detail!);

    expect(content.getByText(asset.id)).toBeInTheDocument();
    expect(content.getByText(asset.kind)).toBeInTheDocument();
    expect(content.getByText(asset.key)).toBeInTheDocument();
    expect(
      content.getByText('First seen').nextElementSibling,
    ).toHaveTextContent(asset.firstSeen);
    expect(content.getByText('Last seen').nextElementSibling).toHaveTextContent(
      asset.lastSeen,
    );
    expect(
      content.getByText(String(asset.observationIds.length)),
    ).toBeInTheDocument();
  });

  it('does not select an asset automatically', async () => {
    render(<App client={clientFor(fixtureSnapshot)} />);

    await screen.findByRole('heading', { name: 'Assets disponibles' });
    expect(
      screen.queryByRole('heading', { name: 'Asset seleccionado' }),
    ).not.toBeInTheDocument();
    for (const asset of fixtureSnapshot.assets) {
      expect(buttonFor(asset)).toHaveAttribute('aria-pressed', 'false');
    }
  });

  it('clears selection when the selected asset disappears', async () => {
    const user = userEvent.setup();
    const selected = fixtureSnapshot.assets[0]!;
    const originalClient = clientFor(fixtureSnapshot);
    const reducedClient = clientFor(withoutAsset(selected.id));
    const view = render(<App client={originalClient} />);

    await user.click(await screen.findByText(selected.key));
    expect(buttonFor(selected)).toHaveAttribute('aria-pressed', 'true');

    view.rerender(<App client={reducedClient} />);
    await waitFor(() => {
      expect(screen.queryByText(selected.key)).not.toBeInTheDocument();
    });

    view.rerender(<App client={originalClient} />);
    await screen.findByText(selected.key);
    expect(buttonFor(selected)).toHaveAttribute('aria-pressed', 'false');
    expect(
      screen.queryByRole('heading', { name: 'Asset seleccionado' }),
    ).not.toBeInTheDocument();
  });

  it('renders hostile imported content as inert text', () => {
    const hostileKey = '<img src=x onerror="alert(1)">';
    const hostileAsset = { ...fixtureSnapshot.assets[0]!, key: hostileKey };
    const view = render(
      <AssetList
        assets={[hostileAsset]}
        selectedAssetId={null}
        onSelect={vi.fn()}
      />,
    );

    expect(screen.getByText(hostileKey)).toBeInTheDocument();
    expect(view.container.querySelector('img')).toBeNull();
  });

  it('never creates links from imported values', () => {
    const importedUrl = 'https://attacker.invalid/path';
    const asset = { ...fixtureSnapshot.assets[0]!, key: importedUrl };
    render(
      <AssetList assets={[asset]} selectedAssetId={null} onSelect={vi.fn()} />,
    );

    expect(screen.getByText(importedUrl)).toBeInTheDocument();
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
  });
});
