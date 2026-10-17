import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import type { ApiClient } from '../api/ApiClient';
import { FixtureApiClient } from '../api/FixtureApiClient';
import { App } from '../App';
import { decodeSnapshot } from '../contracts/decodeSnapshot';
import type { SnapshotDto } from '../contracts/snapshot';
import fixture from '../../../../docs/contracts/examples/snapshot.json';

const fixtureSnapshot = decodeSnapshot(structuredClone(fixture));
const emptySnapshot = decodeSnapshot({
  ...structuredClone(fixture),
  evidence: [],
  observations: [],
  assets: [],
  relationships: [],
  paths: [],
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe('App snapshot states', () => {
  it('shows loading while the client promise is pending', () => {
    const pending = deferred<SnapshotDto>();
    const client: ApiClient = { loadSnapshot: () => pending.promise };

    render(<App client={client} />);

    expect(screen.getByText('Cargando snapshot…')).toBeInTheDocument();
  });

  it('shows empty for a valid snapshot without assets', async () => {
    const client: ApiClient = {
      loadSnapshot: () => Promise.resolve(emptySnapshot),
    };

    render(<App client={client} />);

    expect(
      await screen.findByRole('heading', { name: 'Sin assets' }),
    ).toBeInTheDocument();
  });

  it('shows a sanitized error when the client fails', async () => {
    const client: ApiClient = {
      loadSnapshot: () => Promise.reject(new Error('internal secret')),
    };

    render(<App client={client} />);

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'No se pudo cargar el snapshot.',
    );
    expect(screen.queryByText(/internal secret/i)).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reintentar' })).toBeEnabled();
  });

  it('retries after an error and can recover', async () => {
    const user = userEvent.setup();
    const loadSnapshot = vi
      .fn<ApiClient['loadSnapshot']>()
      .mockRejectedValueOnce(new Error('temporary failure'))
      .mockResolvedValueOnce(fixtureSnapshot);
    const client: ApiClient = { loadSnapshot };

    render(<App client={client} />);
    await user.click(await screen.findByRole('button', { name: 'Reintentar' }));

    expect(
      await screen.findByRole('heading', { name: 'Resumen del snapshot' }),
    ).toBeInTheDocument();
    expect(loadSnapshot).toHaveBeenCalledTimes(2);
  });

  it('shows the minimal success summary using FixtureApiClient', async () => {
    render(<App client={new FixtureApiClient()} />);

    expect(
      await screen.findByRole('heading', { name: 'Resumen del snapshot' }),
    ).toBeInTheDocument();
    expect(screen.getByText('1.0.0')).toBeInTheDocument();
    expect(screen.getByText('Assets').nextElementSibling).toHaveTextContent(
      '4',
    );
    expect(
      screen.getByText('Observations').nextElementSibling,
    ).toHaveTextContent('3');
    expect(screen.getByText('Evidence').nextElementSibling).toHaveTextContent(
      '1',
    );
  });

  it('aborts on unmount and ignores a later resolution', async () => {
    const pending = deferred<SnapshotDto>();
    let receivedSignal: AbortSignal | undefined;
    const client: ApiClient = {
      loadSnapshot: (options) => {
        receivedSignal = options?.signal;
        return pending.promise;
      },
    };
    const view = render(<App client={client} />);

    view.unmount();
    expect(receivedSignal?.aborted).toBe(true);

    await act(async () => {
      pending.resolve(fixtureSnapshot);
      await pending.promise;
    });

    expect(view.container).toBeEmptyDOMElement();
  });
});
