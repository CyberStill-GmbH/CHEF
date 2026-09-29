import type { ServiceInput, ScopePolicy } from './model.js';

export interface IdentityPort {
  digest(bytes: Uint8Array | string): string;
  id(namespace: string, parts: readonly string[]): string;
}
export interface ClockPort {
  now(): string;
}
export interface ImportParser {
  readonly source: 'nmap-xml';
  readonly version: string;
  parse(bytes: Uint8Array, maxObservations: number): ServiceInput[];
}
export interface ScopeGuard {
  check(input: ServiceInput, policy: ScopePolicy): void;
}
