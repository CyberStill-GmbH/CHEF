import { useCallback, useEffect, useState } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type { ReconGraphDto } from '../contracts/recon-graph';
import type { SnapshotDto } from '../contracts/snapshot';
import type { ScanLogDto } from '../contracts/scan-log';

const LOAD_ERROR_MESSAGE = 'No se pudo cargar el snapshot.';

export type SnapshotState =
  | { status: 'loading' }
  | { status: 'empty' }
  | { status: 'error'; message: string }
  | { status: 'success'; snapshot: SnapshotDto; reconGraph?: ReconGraphDto };

export interface UseSnapshotResult {
  state: SnapshotState;
  retry: () => void;
  showSnapshot: (snapshot: SnapshotDto, reconGraph?: ReconGraphDto) => void;
}

export function useSnapshot(
  client: ApiClient,
  onScanLog?: (event: ScanLogDto) => void,
): UseSnapshotResult {
  const [state, setState] = useState<SnapshotState>({ status: 'loading' });
  const [requestNumber, setRequestNumber] = useState(0);

  const retry = useCallback(() => {
    setRequestNumber((current) => current + 1);
  }, []);

  const showSnapshot = useCallback(
    (snapshot: SnapshotDto, reconGraph?: ReconGraphDto) => {
      setState({
        status: 'success',
        snapshot,
        ...(reconGraph === undefined ? {} : { reconGraph }),
      });
    },
    [],
  );

  useEffect(() => {
    const controller = new AbortController();
    setState({ status: 'loading' });

    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        unlisten = await client.listenScanLogs?.(
          onScanLog ?? (() => undefined),
          {
            signal: controller.signal,
          },
        );
        const snapshot = await client.loadSnapshot({
          signal: controller.signal,
        });
        if (controller.signal.aborted) return;

        const reconGraph = await client.loadReconGraph?.(snapshot.run.id, {
          signal: controller.signal,
        });
        if (controller.signal.aborted) return;

        setState(
          snapshot.assets.length === 0 && (reconGraph?.assets.length ?? 0) === 0
            ? { status: 'empty' }
            : {
                status: 'success',
                snapshot,
                ...(reconGraph === undefined ? {} : { reconGraph }),
              },
        );
      } catch {
        if (controller.signal.aborted) return;
        setState({ status: 'error', message: LOAD_ERROR_MESSAGE });
      }
    })();

    return () => {
      controller.abort();
      unlisten?.();
    };
  }, [client, onScanLog, requestNumber]);

  return { state, retry, showSnapshot };
}
