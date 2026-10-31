import { ChefError } from '../../domain/src/errors.js';
import type { ImportParser } from '../../domain/src/ports.js';
import type { ServiceInput } from '../../domain/src/model.js';
import { canonicalAddress } from './address.js';

export interface CommonCrawlParseStats {
  cdxRecords: number;
  kept: number;
  skippedOutOfScope: number;
  skippedInvalidUrl: number;
  skippedNoIp: number;
}

export interface CommonCrawlParseResult {
  inputs: ServiceInput[];
  stats: CommonCrawlParseStats;
}

const CC_INDEX_HOST = 'index.commoncrawl.org';
const CC_DEFAULT_COLLECTION = 'CC-MAIN-2024-51';
const MAX_CDX_RECORDS = 10000;
const REQUEST_TIMEOUT_MS = 30000;
const MAX_RETRIES = 3;
const BASE_BACKOFF_MS = 1000;

interface CdxRecord {
  urlkey: string;
  timestamp: string;
  original: string;
  mimetype: string;
  status: string;
  digest: string;
  length: string;
  offset: string;
  filename: string;
}

export class CommonCrawlIndexParser implements ImportParser {
  readonly source = 'common-crawl-index' as const;
  readonly version = '1.0.0';

  private readonly allowedAddresses: Set<string>;
  private readonly allowedPorts: Set<number>;
  private readonly allowedProtocols: Set<'tcp' | 'udp'>;

  constructor(
    allowedAddresses: readonly string[],
    allowedPorts: readonly number[],
    allowedProtocols: readonly ('tcp' | 'udp')[],
  ) {
    this.allowedAddresses = new Set(allowedAddresses.map(canonicalAddress));
    this.allowedPorts = new Set(allowedPorts);
    this.allowedProtocols = new Set(allowedProtocols);
  }

  parse(bytes: Uint8Array, maxObservations: number): ServiceInput[] {
    return this.parseInternal(bytes, maxObservations).inputs;
  }

  parseWithStats(
    bytes: Uint8Array,
    maxObservations: number,
  ): CommonCrawlParseResult {
    return this.parseInternal(bytes, maxObservations);
  }

  /** Fetch CDX index for a given scope address. Returns raw CDX lines. */
  static async fetchCdxForAddress(
    address: string,
    collection: string = CC_DEFAULT_COLLECTION,
    signal?: AbortSignal,
  ): Promise<string> {
    const url = new URL(`https://${CC_INDEX_HOST}/${collection}-index`);
    url.searchParams.set('url', address);
    url.searchParams.set('output', 'json');
    url.searchParams.set('limit', String(MAX_CDX_RECORDS));
    url.searchParams.set('filter', 'status:200');

    let lastError: Error | null = null;
    for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(
          () => controller.abort(),
          REQUEST_TIMEOUT_MS,
        );
        if (signal) signal.addEventListener('abort', () => controller.abort());

        const response = await fetch(url.toString(), {
          method: 'GET',
          headers: {
            'User-Agent':
              'CHEF/1.0.0 (https://github.com/chef-evidence-core; +commoncrawl.org)',
            Accept: 'application/json',
          },
          signal: controller.signal,
        });
        clearTimeout(timeoutId);

        if (response.status === 429) {
          const retryAfter = response.headers.get('Retry-After');
          const waitMs = retryAfter
            ? parseInt(retryAfter, 10) * 1000
            : BASE_BACKOFF_MS * 2 ** attempt;
          await new Promise((r) => setTimeout(r, Math.min(waitMs, 60000)));
          continue;
        }
        if (!response.ok) {
          throw new ChefError(
            'UPSTREAM_ERROR',
            `Common Crawl CDX ${response.status}: ${response.statusText}`,
          );
        }
        return await response.text();
      } catch (e) {
        lastError = e instanceof Error ? e : new Error(String(e));
        if (e instanceof ChefError) throw e;
        if (attempt === MAX_RETRIES) break;
        await new Promise((r) => setTimeout(r, BASE_BACKOFF_MS * 2 ** attempt));
      }
    }
    throw new ChefError(
      'UPSTREAM_UNAVAILABLE',
      `Common Crawl CDX unavailable after retries: ${lastError?.message}`,
    );
  }

  /** Parse raw CDX JSON lines into ServiceInput[]. */
  private parseInternal(
    bytes: Uint8Array,
    maxObservations: number,
  ): CommonCrawlParseResult {
    const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    const lines = text
      .trim()
      .split('\n')
      .filter((l) => l.length > 0);

    const stats: CommonCrawlParseStats = {
      cdxRecords: lines.length,
      kept: 0,
      skippedOutOfScope: 0,
      skippedInvalidUrl: 0,
      skippedNoIp: 0,
    };

    const inputs: ServiceInput[] = [];

    for (const line of lines) {
      if (inputs.length >= maxObservations) break;

      let record: CdxRecord;
      try {
        record = JSON.parse(line) as CdxRecord;
      } catch {
        stats.skippedInvalidUrl += 1;
        continue;
      }

      const originalUrl = record.original;
      let parsedUrl: URL;
      try {
        parsedUrl = new URL(originalUrl);
      } catch {
        stats.skippedInvalidUrl += 1;
        continue;
      }

      const hostname = parsedUrl.hostname.replace(/^\[(.*)\]$/, '$1');
      let address: string;
      try {
        address = canonicalAddress(hostname);
      } catch {
        stats.skippedNoIp += 1;
        continue;
      }

      if (record.status !== '200') {
        stats.skippedOutOfScope += 1;
        continue;
      }

      if (!this.allowedAddresses.has(address)) {
        stats.skippedOutOfScope += 1;
        continue;
      }

      const port = parsedUrl.port
        ? parseInt(parsedUrl.port, 10)
        : parsedUrl.protocol === 'https:'
          ? 443
          : 80;
      const protocol = 'tcp';

      if (
        !this.allowedPorts.has(port) ||
        !this.allowedProtocols.has(protocol)
      ) {
        stats.skippedOutOfScope += 1;
        continue;
      }

      const observedAt = new Date(
        parseInt(record.timestamp.slice(0, 14), 10),
      ).toISOString();

      inputs.push({
        address,
        port,
        protocol,
        observedAt,
        locator: `/cdx/${record.filename}/${record.offset}/${record.length}`,
      });

      stats.kept += 1;
    }

    return { inputs, stats };
  }
}

/** Factory to create parser from ScopePolicy (used by ImportEvidence). */
export function createCommonCrawlParser(policy: {
  allowedAddresses: readonly string[];
  allowedPorts: readonly number[];
  allowedProtocols: readonly ('tcp' | 'udp')[];
}): CommonCrawlIndexParser {
  return new CommonCrawlIndexParser(
    policy.allowedAddresses,
    policy.allowedPorts,
    policy.allowedProtocols,
  );
}
