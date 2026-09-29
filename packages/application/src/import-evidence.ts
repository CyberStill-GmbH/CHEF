import { ChefError } from '../../domain/src/errors.js';
import { correlate } from '../../domain/src/correlate.js';
import {
  SCHEMA_VERSION,
  type ScopePolicy,
  type Snapshot,
} from '../../domain/src/model.js';
import type {
  ClockPort,
  IdentityPort,
  ImportParser,
  ScopeGuard,
} from '../../domain/src/ports.js';

export class ImportEvidence {
  constructor(
    private readonly parser: ImportParser,
    private readonly scope: ScopeGuard,
    private readonly identity: IdentityPort,
    private readonly clock: ClockPort,
  ) {}

  execute(
    bytes: Uint8Array,
    policy: ScopePolicy,
    signal?: AbortSignal,
  ): Snapshot {
    const checkCancellation = () => {
      if (signal?.aborted) throw new ChefError('CANCELLED', 'Import cancelled');
    };
    checkCancellation();
    if (bytes.byteLength > policy.maxInputBytes)
      throw new ChefError('LIMIT_EXCEEDED', 'Input exceeds scope byte limit');
    const startedAt = this.clock.now();
    const parsed = this.parser.parse(bytes, policy.maxObservations);
    const sha256 = this.identity.digest(bytes);
    const scopeDigest = this.identity.digest(JSON.stringify(policy));
    const runId = this.identity.id('run', [
      sha256,
      scopeDigest,
      this.parser.version,
    ]);
    const evidenceId = this.identity.id('evidence', [runId, sha256]);
    // Build only after validation of all records: failed imports cannot publish partial graphs.
    for (const input of parsed) {
      checkCancellation();
      this.scope.check(input, policy);
    }
    const observations = parsed
      .map((input) => ({
        ...input,
        id: this.identity.id('observation', [evidenceId, input.locator]),
        evidenceId,
        runId,
        source: this.parser.source,
        fact: 'open-port' as const,
        confidence: 1,
      }))
      .sort((a, b) => a.id.localeCompare(b.id));
    const graph = correlate(observations, this.identity);
    checkCancellation();
    return {
      schemaVersion: SCHEMA_VERSION,
      scopePolicy: structuredClone(policy),
      run: {
        id: runId,
        scopePolicyId: policy.id,
        scopeDigest,
        state: 'completed',
        startedAt,
        finishedAt: this.clock.now(),
      },
      evidence: [
        {
          id: evidenceId,
          sha256,
          source: this.parser.source,
          parserVersion: this.parser.version,
          runId,
          locator: '/nmaprun',
          importedAt: startedAt,
        },
      ],
      observations,
      ...graph,
    };
  }
}
