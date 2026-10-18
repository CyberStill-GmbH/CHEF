import { useCallback, useEffect, useState } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type { SnapshotDto } from '../contracts/snapshot';

const LOAD_ERROR_MESSAGE = 'No se pudo cargar el snapshot.';

export type SnapshotState =
  | { status: 'loading' }
  | { status: 'empty' }
  | { status: 'error'; message: string }
  | { status: 'success'; snapshot: SnapshotDto };

export interface UseSnapshotResult {
  state: SnapshotState;
  retry: () => void;
}

export function useSnapshot(client: ApiClient): UseSnapshotResult {
  const [state, setState] = useState<SnapshotState>({ status: 'loading' });
  const [requestNumber, setRequestNumber] = useState(0);

  const retry = useCallback(() => {
    setRequestNumber((current) => current + 1);
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    setState({ status: 'loading' });

    void client
      .loadSnapshot({ signal: controller.signal })
      .then((snapshot) => {
        if (controller.signal.aborted) return;

        setState(
          snapshot.assets.length === 0
            ? { status: 'empty' }
            : { status: 'success', snapshot },
        );
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setState({ status: 'error', message: LOAD_ERROR_MESSAGE });
      });

    return () => {
      controller.abort();
    };
  }, [client, requestNumber]);

  return { state, retry };
}
