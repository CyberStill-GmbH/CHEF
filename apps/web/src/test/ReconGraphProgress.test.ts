import { describe, expect, it } from 'vitest';

import { decodeReconGraphProgress } from '../contracts/decodeReconGraphProgress';

const completed = {
  schemaVersion: '1.0.0',
  requestId: 'request-1',
  jobId: 'job-1',
  sequence: 1,
  phase: 'completed',
  assets: [],
  evidence: [],
  relationships: [],
  unmappedEventTypes: { TXT: 2 },
};

describe('decodeReconGraphProgress', () => {
  it('accepts a bounded terminal event after a validated projection', () => {
    expect(decodeReconGraphProgress(completed)).toEqual(completed);
  });

  it('rejects mixed entity families in a single batch phase', () => {
    expect(() =>
      decodeReconGraphProgress({
        ...completed,
        phase: 'assets',
        assets: [
          {
            id: 'asset-1',
            kind: 'domain',
            key: 'example.invalid',
            qualification: null,
            firstSeen: '2026-10-18T00:00:00.000Z',
            lastSeen: '2026-10-18T00:00:00.000Z',
            evidenceIds: ['evidence-1'],
          },
        ],
        evidence: [{ id: 'evidence-1' }],
      }),
    ).toThrow();
  });

  it('rejects batches above the contract limit', () => {
    expect(() =>
      decodeReconGraphProgress({
        ...completed,
        phase: 'assets',
        assets: Array.from({ length: 101 }, (_, index) => ({
          id: `asset-${index}`,
          kind: 'domain',
          key: 'example.invalid',
          qualification: null,
          firstSeen: '2026-10-18T00:00:00.000Z',
          lastSeen: '2026-10-18T00:00:00.000Z',
          evidenceIds: ['evidence-1'],
        })),
        unmappedEventTypes: {},
      }),
    ).toThrow();
  });
});
