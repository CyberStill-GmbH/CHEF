import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { isAbsolute, join } from 'node:path';
import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';

import { ChefError } from '../../domain/src/errors.js';
import type {
  Asset,
  CorrelationGraph,
  PreparedImport,
} from '../../domain/src/model.js';
import type { CorrelationEngine } from '../../domain/src/ports.js';
import { Sha256Identity } from './identity.js';

interface WireResult {
  schemaVersion: 'correlation-result/v0';
  ruleSetVersion: string;
  asOf: string;
  assets: Asset[];
  relationships: {
    id: string;
    from: string;
    to: string;
    kind: 'exposes';
    status: string;
    observationIds: string[];
    confidence: number;
  }[];
  paths: {
    id: string;
    assetIds: string[];
    relationshipIds: string[];
    ruleId: 'open-service/v1';
    ruleVersion: 'v1';
    status: string;
    explanation: string;
    confidence: number;
    isConfirmedVulnerability: false;
  }[];
  metrics: {
    observationsIn: number;
    assetsOut: number;
    relationshipsOut: number;
    pathsOut: number;
    duplicateObservations: number;
  };
}

export interface GoCorrelationOptions {
  /** Trusted deployment configuration, never a request field. */
  executable: string;
  args?: readonly string[];
  contractsDirectory?: string;
  timeoutMs?: number;
  maxInputBytes?: number;
  maxOutputBytes?: number;
  maxConcurrent?: number;
}

/** Local JSON transport. No shell, inherited credentials, or partial results. */
export class GoCorrelationEngine implements CorrelationEngine {
  private active = 0;
  private readonly timeoutMs: number;
  private readonly maxInputBytes: number;
  private readonly maxOutputBytes: number;
  private readonly maxConcurrent: number;
  private readonly validateJob;
  private readonly validateResult;

  constructor(private readonly options: GoCorrelationOptions) {
    if (!isAbsolute(options.executable))
      throw new Error('Go executable must be an absolute path');
    this.timeoutMs = options.timeoutMs ?? 10_000;
    this.maxInputBytes = options.maxInputBytes ?? 8 * 1024 * 1024;
    this.maxOutputBytes = options.maxOutputBytes ?? 8 * 1024 * 1024;
    this.maxConcurrent = options.maxConcurrent ?? 2;
    for (const value of [
      this.timeoutMs,
      this.maxInputBytes,
      this.maxOutputBytes,
      this.maxConcurrent,
    ])
      if (!Number.isSafeInteger(value) || value <= 0)
        throw new Error('Invalid Go execution limit');
    if (this.maxInputBytes > 8 * 1024 * 1024)
      throw new Error('Go input limit exceeds engine limit');
    const ajv = new Ajv2020({ strict: true });
    addFormatsModule.default(ajv);
    const directory = options.contractsDirectory ?? 'docs/contracts';
    this.validateJob = ajv.compile(
      JSON.parse(
        readFileSync(join(directory, 'correlation-job.schema.json'), 'utf8'),
      ) as object,
    );
    this.validateResult = ajv.compile<WireResult>(
      JSON.parse(
        readFileSync(join(directory, 'correlation-result.schema.json'), 'utf8'),
      ) as object,
    );
  }

  async correlate(
    input: PreparedImport,
    signal?: AbortSignal,
  ): Promise<CorrelationGraph> {
    if (signal?.aborted)
      throw new ChefError('CANCELLED', 'Correlation cancelled');
    const { scopePolicy: policy } = input;
    const job = {
      schemaVersion: 'correlation-job/v0',
      ruleSetVersion: 'v1',
      asOf: input.run.finishedAt,
      projectScope: {
        id: policy.id,
        digest: input.run.scopeDigest,
        allowedAddresses: policy.allowedAddresses,
        excludedAddresses: policy.excludedAddresses,
        allowedPorts: policy.allowedPorts,
        allowedProtocols: policy.allowedProtocols,
      },
      evidence: input.evidence.map(
        ({ id, sha256, source, locator, importedAt }) => ({
          id,
          sha256,
          source,
          locator,
          importedAt,
        }),
      ),
      observations: input.observations,
    };
    if (!this.validateJob(job))
      throw new ChefError(
        'INVALID_INPUT',
        'Correlation job fails schema validation',
      );
    const evidence = new Map(input.evidence.map((e) => [e.id, e]));
    if (
      evidence.size !== input.evidence.length ||
      new Set(input.observations.map((o) => o.id)).size !==
        input.observations.length ||
      input.observations.some((o) => {
        const e = evidence.get(o.evidenceId);
        return (
          !e ||
          e.runId !== o.runId ||
          e.source !== o.source ||
          !policy.allowedAddresses.includes(o.address) ||
          policy.excludedAddresses.includes(o.address) ||
          !policy.allowedPorts.includes(o.port) ||
          !policy.allowedProtocols.includes(o.protocol)
        );
      })
    )
      throw new ChefError(
        'INVALID_INPUT',
        'Correlation job has invalid scope or references',
      );
    const payload = JSON.stringify(job);
    if (Buffer.byteLength(payload) > this.maxInputBytes)
      throw new ChefError('LIMIT_EXCEEDED', 'Correlation input exceeds limit');
    if (this.active >= this.maxConcurrent)
      throw new ChefError('UPSTREAM_UNAVAILABLE', 'Correlation engine is busy');
    this.active++;
    try {
      const raw = await this.execute(payload, signal);
      let result: unknown;
      try {
        result = JSON.parse(raw);
      } catch {
        throw new ChefError(
          'UPSTREAM_ERROR',
          'Correlation engine returned invalid JSON',
        );
      }
      if (
        !this.validateResult(result) ||
        result.ruleSetVersion !== 'v1' ||
        result.asOf !== job.asOf
      )
        throw new ChefError(
          'UPSTREAM_ERROR',
          'Correlation result fails contract validation',
        );
      this.checkReferences(input, result);
      return {
        assets: result.assets,
        relationships: result.relationships.map((r) => ({
          id: r.id,
          from: r.from,
          to: r.to,
          kind: r.kind,
          observationIds: r.observationIds,
          confidence: r.confidence,
          assertion: 'observed',
        })),
        paths: result.paths.map((p) => ({
          id: p.id,
          assetIds: p.assetIds,
          relationshipIds: p.relationshipIds,
          ruleId: p.ruleId,
          explanation: p.explanation,
          confidence: p.confidence,
          isConfirmedVulnerability: p.isConfirmedVulnerability,
          assertion: 'inferred',
        })),
      };
    } finally {
      this.active--;
    }
  }

  private execute(payload: string, signal?: AbortSignal): Promise<string> {
    // Go needs no provider/session/database credentials. Windows requires SystemRoot.
    const env: NodeJS.ProcessEnv = {};
    for (const key of ['SystemRoot', 'SYSTEMROOT', 'WINDIR', 'TEMP', 'TMP'])
      if (process.env[key] !== undefined) env[key] = process.env[key];
    return new Promise((resolve, reject) => {
      const child = spawn(
        this.options.executable,
        [...(this.options.args ?? [])],
        {
          shell: false,
          windowsHide: true,
          env,
          stdio: ['pipe', 'pipe', 'pipe'],
        },
      );
      let failure: ChefError | undefined;
      let stdoutBytes = 0,
        stderrBytes = 0;
      const chunks: Buffer[] = [];
      const stop = (error: ChefError) => {
        failure ??= error;
        child.kill('SIGKILL');
      };
      const cancel = () =>
        stop(new ChefError('CANCELLED', 'Correlation cancelled'));
      const timer = setTimeout(
        () =>
          stop(new ChefError('UPSTREAM_ERROR', 'Correlation engine timed out')),
        this.timeoutMs,
      );
      signal?.addEventListener('abort', cancel, { once: true });
      if (signal?.aborted) cancel();
      child.stdout.on('data', (chunk: Buffer) => {
        stdoutBytes += chunk.length;
        if (stdoutBytes > this.maxOutputBytes)
          stop(
            new ChefError('UPSTREAM_ERROR', 'Correlation output exceeds limit'),
          );
        else if (!failure) chunks.push(chunk);
      });
      child.stderr.on('data', (chunk: Buffer) => {
        stderrBytes += chunk.length;
        if (stderrBytes > this.maxOutputBytes)
          stop(
            new ChefError(
              'UPSTREAM_ERROR',
              'Correlation diagnostics exceed limit',
            ),
          );
      });
      child.on('error', () => {
        failure ??= new ChefError(
          'UPSTREAM_ERROR',
          'Correlation engine could not start',
        );
      });
      // Release the concurrency slot only after the process and its streams close.
      child.on('close', (code) => {
        clearTimeout(timer);
        signal?.removeEventListener('abort', cancel);
        if (failure) reject(failure);
        else if (code !== 0)
          reject(new ChefError('UPSTREAM_ERROR', 'Correlation engine failed'));
        else resolve(Buffer.concat(chunks).toString('utf8'));
      });
      // EPIPE can follow an early child exit; the completion callback owns the error.
      child.stdin?.on('error', () => {});
      child.stdin?.end(payload);
    });
  }

  private checkReferences(input: PreparedImport, result: WireResult): void {
    const identity = new Sha256Identity();
    const invalid = () => {
      throw new ChefError(
        'UPSTREAM_ERROR',
        'Correlation result has invalid references or metrics',
      );
    };
    const observations = new Map(input.observations.map((o) => [o.id, o]));
    const assets = new Map(result.assets.map((a) => [a.id, a]));
    const relationships = new Map(result.relationships.map((r) => [r.id, r]));
    const used = new Set<string>();
    if (
      assets.size !== result.assets.length ||
      relationships.size !== result.relationships.length ||
      new Set(result.paths.map((p) => p.id)).size !== result.paths.length
    )
      invalid();
    for (const asset of result.assets) {
      if (
        asset.observationIds.length === 0 ||
        new Set(asset.observationIds).size !== asset.observationIds.length ||
        asset.id !== identity.id('asset', [asset.kind, asset.key])
      )
        invalid();
      for (const id of asset.observationIds) {
        const o = observations.get(id);
        if (
          !o ||
          asset.key !==
            (asset.kind === 'address'
              ? o.address
              : `${o.address}|${o.protocol}|${o.port}`)
        )
          invalid();
        used.add(id);
      }
    }
    if (used.size !== observations.size) invalid();
    for (const r of result.relationships) {
      const from = assets.get(r.from),
        to = assets.get(r.to);
      if (
        r.status !== 'observed' ||
        from?.kind !== 'address' ||
        to?.kind !== 'service' ||
        r.id !== identity.id('relationship', ['exposes', r.from, r.to]) ||
        r.observationIds.length === 0 ||
        r.observationIds.some(
          (id) =>
            !from.observationIds.includes(id) ||
            !to.observationIds.includes(id),
        )
      )
        invalid();
    }
    for (const p of result.paths)
      if (
        p.status !== 'inferred' ||
        p.assetIds.length !== 2 ||
        p.relationshipIds.length !== 1 ||
        p.id !==
          identity.id('path', [
            'open-service/v1',
            p.relationshipIds[0] ?? '',
          ]) ||
        p.relationshipIds.some((id) => {
          const r = relationships.get(id);
          return !r || p.assetIds[0] !== r.from || p.assetIds[1] !== r.to;
        })
      )
        invalid();
    const m = result.metrics;
    if (
      m.observationsIn !== observations.size ||
      m.assetsOut !== assets.size ||
      m.relationshipsOut !== relationships.size ||
      m.pathsOut !== result.paths.length ||
      result.paths.length !== relationships.size ||
      relationships.size !==
        result.assets.filter((a) => a.kind === 'service').length ||
      m.duplicateObservations !==
        observations.size -
          result.assets.filter((a) => a.kind === 'service').length
    )
      invalid();
  }
}
