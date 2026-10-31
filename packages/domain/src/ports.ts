import type { ServiceInput, ScopePolicy, Snapshot } from './model.js';
import type { Membership, Project, Role } from './project.js';

export interface IdentityPort {
  digest(bytes: Uint8Array | string): string;
  id(namespace: string, parts: readonly string[]): string;
}
export interface ClockPort {
  now(): string;
}
export interface RandomPort {
  /** Cryptographically strong, URL-safe, at least `bytes` bytes of entropy. */
  token(bytes: number): string;
}
export interface ImportParser {
  readonly source: 'nmap-xml' | 'common-crawl-index';
  readonly version: string;
  parse(bytes: Uint8Array, maxObservations: number): ServiceInput[];
}
export interface ScopeGuard {
  check(input: ServiceInput, policy: ScopePolicy): void;
}

export interface ProjectSummary {
  project: Project;
  role: Role;
}

/** Membership is stored separately so a project never carries its own ACL copy. */
export interface ProjectRepository {
  saveProject(project: Project): Promise<void>;
  findProject(id: string): Promise<Project | undefined>;
  listProjectsForUser(userId: string): Promise<ProjectSummary[]>;
  saveMembership(membership: Membership): Promise<void>;
  findMembership(
    projectId: string,
    userId: string,
  ): Promise<Membership | undefined>;
}

export interface SnapshotStore {
  save(projectId: string, snapshot: Snapshot): Promise<void>;
  latest(projectId: string): Promise<Snapshot | undefined>;
}

export interface SessionService {
  issue(userId: string): string;
  /** Returns the user id or throws; never returns an empty identity. */
  verify(token: string): string;
  issueNonce(purpose: string): string;
  verifyNonce(nonce: string, purpose: string): void;
}

export interface OAuthIdentity {
  providerUserId: string;
  login: string;
}

export interface OAuthProvider {
  readonly name: string;
  authorizeUrl(state: string): string;
  exchange(code: string): Promise<OAuthIdentity>;
}
