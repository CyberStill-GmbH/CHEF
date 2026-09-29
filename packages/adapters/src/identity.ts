import { createHash } from 'node:crypto';
import type { IdentityPort } from '../../domain/src/ports.js';
export class Sha256Identity implements IdentityPort {
  digest(bytes: Uint8Array | string): string {
    return createHash('sha256').update(bytes).digest('hex');
  }
  id(namespace: string, parts: readonly string[]): string {
    return `${namespace}:${this.digest(JSON.stringify(parts))}`;
  }
}
