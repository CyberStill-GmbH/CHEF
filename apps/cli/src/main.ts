import { open } from 'node:fs/promises';
import { resolve } from 'node:path';
import { ImportEvidence } from '../../../packages/application/src/import-evidence.js';
import { NmapXmlParser } from '../../../packages/adapters/src/nmap.js';
import { ExactScopeGuard } from '../../../packages/adapters/src/scope.js';
import { Sha256Identity } from '../../../packages/adapters/src/identity.js';
import { loadContracts } from '../../../packages/adapters/src/contracts.js';
import { ChefError } from '../../../packages/domain/src/errors.js';

async function boundedRead(path: string, limit: number): Promise<Uint8Array> {
  const file = await open(path, 'r');
  try {
    const stat = await file.stat();
    if (!stat.isFile() || stat.size > limit)
      throw new ChefError(
        'LIMIT_EXCEEDED',
        'Input is not a regular bounded file',
      );
    // Read at most limit+1, even if the file grows after stat.
    const buffer = Buffer.alloc(limit + 1);
    let offset = 0;
    while (offset < buffer.length) {
      const { bytesRead } = await file.read(
        buffer,
        offset,
        buffer.length - offset,
        null,
      );
      if (!bytesRead) break;
      offset += bytesRead;
    }
    if (offset > limit)
      throw new ChefError('LIMIT_EXCEEDED', 'File grew beyond input limit');
    return buffer.subarray(0, offset);
  } finally {
    await file.close();
  }
}

try {
  const [input, scopeFile, ...extra] = process.argv.slice(2);
  if (!input || !scopeFile || extra.length)
    throw new ChefError(
      'INVALID_INPUT',
      'Usage: node dist/apps/cli/src/main.js <nmap.xml> <scope.json>',
    );
  const contracts = loadContracts(
    resolve('docs/contracts/snapshot.schema.json'),
  );
  const scopeBytes = await boundedRead(scopeFile, 64 * 1024);
  const policy = contracts.scope(
    JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(scopeBytes)),
  );
  const controller = new AbortController();
  process.once('SIGINT', () => controller.abort());
  const bytes = await boundedRead(input, policy.maxInputBytes);
  const pipeline = new ImportEvidence(
    new NmapXmlParser(),
    new ExactScopeGuard(),
    new Sha256Identity(),
    { now: () => new Date().toISOString() },
  );
  const result = pipeline.execute(bytes, policy, controller.signal);
  contracts.snapshot(result);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
} catch (error: unknown) {
  // Do not print target data, paths, XML fragments, or stack traces to routine logs.
  const code = error instanceof ChefError ? error.code : 'INVALID_INPUT';
  process.stderr.write(
    `${JSON.stringify({ code, message: error instanceof ChefError ? error.message : 'Unable to import local evidence' })}\n`,
  );
  process.exitCode = code === 'CANCELLED' ? 130 : 1;
}
