import fixture from '../../../../docs/contracts/examples/snapshot.json';
import { decodeSnapshot } from '../contracts/decodeSnapshot';
import type { ApiClient } from './ApiClient';

export class FixtureApiClient implements ApiClient {
  async loadSnapshot(options?: { signal?: AbortSignal }) {
    options?.signal?.throwIfAborted();
    return decodeSnapshot(fixture);
  }
}
