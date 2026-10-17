import { useEffect, useState } from 'react';

import type { ApiClient } from './api/ApiClient';
import { FixtureApiClient } from './api/FixtureApiClient';
import { AssetList } from './components/AssetList';
import { EmptyState } from './components/EmptyState';
import { ErrorState } from './components/ErrorState';
import { EvidencePanel } from './components/EvidencePanel';
import { LoadingState } from './components/LoadingState';
import { useSnapshot } from './hooks/useSnapshot';

const defaultClient = new FixtureApiClient();

interface AppProps {
  client?: ApiClient;
}

export function App({ client = defaultClient }: AppProps) {
  const { state, retry } = useSnapshot(client);
  const [selectedAssetId, setSelectedAssetId] = useState<string | null>(null);

  useEffect(() => {
    if (state.status === 'empty') {
      setSelectedAssetId(null);
      return;
    }

    if (
      state.status === 'success' &&
      selectedAssetId !== null &&
      !state.snapshot.assets.some(({ id }) => id === selectedAssetId)
    ) {
      setSelectedAssetId(null);
    }
  }, [selectedAssetId, state]);

  const selectedAsset =
    state.status === 'success'
      ? state.snapshot.assets.find(({ id }) => id === selectedAssetId)
      : undefined;

  return (
    <main>
      <h1>CHEF Frontend Smoke</h1>
      {state.status === 'loading' && <LoadingState />}
      {state.status === 'empty' && <EmptyState />}
      {state.status === 'error' && (
        <ErrorState message={state.message} onRetry={retry} />
      )}
      {state.status === 'success' && (
        <>
          <section aria-labelledby="snapshot-summary-title">
            <h2 id="snapshot-summary-title">Resumen del snapshot</h2>
            <dl>
              <dt>Versión</dt>
              <dd>{state.snapshot.schemaVersion}</dd>
              <dt>Assets</dt>
              <dd>{state.snapshot.assets.length}</dd>
              <dt>Observations</dt>
              <dd>{state.snapshot.observations.length}</dd>
              <dt>Evidence</dt>
              <dd>{state.snapshot.evidence.length}</dd>
            </dl>
          </section>
          <AssetList
            assets={state.snapshot.assets}
            selectedAssetId={selectedAssetId}
            onSelect={setSelectedAssetId}
          />
          {selectedAsset !== undefined && (
            <EvidencePanel snapshot={state.snapshot} asset={selectedAsset} />
          )}
        </>
      )}
    </main>
  );
}
