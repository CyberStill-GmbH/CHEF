import { randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';

import { loadContracts } from '../../../packages/adapters/src/contracts.js';
import { GitHubOAuth } from '../../../packages/adapters/src/github-oauth.js';
import { Sha256Identity } from '../../../packages/adapters/src/identity.js';
import {
  InMemoryProjectRepository,
  InMemorySnapshotStore,
} from '../../../packages/adapters/src/memory-projects.js';
import { NmapXmlParser } from '../../../packages/adapters/src/nmap.js';
import { ExactScopeGuard } from '../../../packages/adapters/src/scope.js';
import { SessionTokenService } from '../../../packages/adapters/src/session-token.js';
import { CreateProject } from '../../../packages/application/src/create-project.js';
import { ImportEvidence } from '../../../packages/application/src/import-evidence.js';
import { ImportIntoProject } from '../../../packages/application/src/import-into-project.js';
import { ListProjects } from '../../../packages/application/src/list-projects.js';
import { ReadProjectSnapshot } from '../../../packages/application/src/read-project-snapshot.js';
import type {
  ClockPort,
  RandomPort,
} from '../../../packages/domain/src/ports.js';
import { createChefApi } from './server.js';

const SCOPE_PATH = process.env['CHEF_SCOPE'] ?? 'fixtures/scope/lab.json';
const PORT = Number(process.env['PORT'] ?? '8787');

function required(name: string): string {
  const value = process.env[name];
  if (value === undefined || value.trim() === '')
    throw new Error(`Missing required environment variable ${name}`);
  return value;
}

const clock: ClockPort = { now: () => new Date().toISOString() };
const random: RandomPort = {
  token: (bytes) => randomBytes(bytes).toString('base64url'),
};

const contracts = loadContracts('docs/contracts/snapshot.schema.json');
const scopePolicy = contracts.scope(
  JSON.parse(readFileSync(SCOPE_PATH, 'utf8')),
);

const projects = new InMemoryProjectRepository();
const snapshots = new InMemorySnapshotStore();
const identity = new Sha256Identity();

const api = createChefApi({
  createProject: new CreateProject(projects, identity, clock, random),
  listProjects: new ListProjects(projects),
  importIntoProject: new ImportIntoProject(
    projects,
    snapshots,
    new ImportEvidence(
      new NmapXmlParser(),
      new ExactScopeGuard(),
      identity,
      clock,
    ),
  ),
  readSnapshot: new ReadProjectSnapshot(projects, snapshots),
  sessions: new SessionTokenService({
    secret: required('CHEF_SESSION_SECRET'),
    nowMs: () => Date.now(),
  }),
  oauth: new GitHubOAuth({
    clientId: required('GITHUB_CLIENT_ID'),
    clientSecret: required('GITHUB_CLIENT_SECRET'),
    redirectUri: required('GITHUB_REDIRECT_URI'),
  }),
  scopePolicy,
});

api.listen(PORT, '127.0.0.1', () => {
  // No secrets, tokens or scope contents are printed on purpose.
  console.log(`CHEF API listening on http://127.0.0.1:${PORT}`);
});

for (const signal of ['SIGINT', 'SIGTERM'] as const)
  process.on(signal, () => {
    api.close(() => process.exit(0));
  });
