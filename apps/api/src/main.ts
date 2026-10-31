import { randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { loadContracts } from '../../../packages/adapters/src/contracts.js';
import { GitHubOAuth } from '../../../packages/adapters/src/github-oauth.js';
import { GoCorrelationEngine } from '../../../packages/adapters/src/go-correlation.js';
import { Sha256Identity } from '../../../packages/adapters/src/identity.js';
import {
  InMemoryProjectRepository,
  InMemorySnapshotStore,
} from '../../../packages/adapters/src/memory-projects.js';
import { NmapXmlParser } from '../../../packages/adapters/src/nmap.js';
import {
  PrismaProjectRepository,
  PrismaSnapshotStore,
  type PrismaLikeClient,
} from '../../../packages/adapters/src/prisma-projects.js';
import { ExactScopeGuard } from '../../../packages/adapters/src/scope.js';
import { SessionTokenService } from '../../../packages/adapters/src/session-token.js';
import { CreateProject } from '../../../packages/application/src/create-project.js';
import { ImportEvidence } from '../../../packages/application/src/import-evidence.js';
import { ImportIntoProject } from '../../../packages/application/src/import-into-project.js';
import { ListProjects } from '../../../packages/application/src/list-projects.js';
import { ReadProjectSnapshot } from '../../../packages/application/src/read-project-snapshot.js';
import type {
  ClockPort,
  ProjectRepository,
  RandomPort,
  SnapshotStore,
} from '../../../packages/domain/src/ports.js';
import { createChefApi } from './server.js';

const SCOPE_PATH = process.env['CHEF_SCOPE'] ?? 'fixtures/scope/lab.json';
const PORT = Number(process.env['PORT'] ?? '8787');
// Por defecto solo loopback; en contenedor se pone 0.0.0.0 para que el puerto
// publicado (acotado a 127.0.0.1 del host en compose) sea alcanzable.
const HOST = process.env['HOST'] ?? '127.0.0.1';

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

interface Persistence {
  projects: ProjectRepository;
  snapshots: SnapshotStore;
  /** Libera recursos (p. ej. conexiones de base de datos) al apagar. */
  dispose: () => Promise<void>;
}

/**
 * Elige la persistencia según el entorno, sin que el dominio sepa cuál es.
 * Sin DATABASE_URL la API corre en memoria (demo offline y pruebas); con
 * DATABASE_URL compone el adaptador Prisma contra PostgreSQL. El cliente Prisma
 * se importa de forma diferida para no exigir `prisma generate` cuando no hay
 * base de datos configurada.
 */
async function createPersistence(): Promise<Persistence> {
  const databaseUrl = process.env['DATABASE_URL'];
  if (databaseUrl === undefined || databaseUrl.trim() === '')
    return {
      projects: new InMemoryProjectRepository(),
      snapshots: new InMemorySnapshotStore(),
      dispose: () => Promise.resolve(),
    };

  const { PrismaClient } = await import('@prisma/client');
  const client = new PrismaClient();
  // El cliente real satisface estructuralmente el puerto mínimo del adaptador.
  const likeClient = client as unknown as PrismaLikeClient;
  return {
    projects: new PrismaProjectRepository(likeClient),
    snapshots: new PrismaSnapshotStore(likeClient, clock),
    dispose: () => client.$disconnect(),
  };
}

async function main(): Promise<void> {
  if (process.env['NODE_ENV'] === 'production') {
    required('DATABASE_URL');
    required('CHEF_ALLOWED_GITHUB_USER_IDS');
  }
  const configuredIds = process.env['CHEF_ALLOWED_GITHUB_USER_IDS']
    ?.split(',')
    .map((id) => id.trim())
    .filter(Boolean);
  if (configuredIds?.some((id) => !/^[1-9][0-9]*$/.test(id)))
    throw new Error(
      'CHEF_ALLOWED_GITHUB_USER_IDS must contain numeric GitHub user IDs',
    );
  const allowedOAuthUserIds = configuredIds?.length
    ? new Set(configuredIds)
    : undefined;
  if (process.env['NODE_ENV'] === 'production' && !allowedOAuthUserIds)
    throw new Error(
      'CHEF_ALLOWED_GITHUB_USER_IDS requires at least one user ID',
    );
  const contracts = loadContracts('docs/contracts/snapshot.schema.json');
  const scopePolicy = contracts.scope(
    JSON.parse(readFileSync(SCOPE_PATH, 'utf8')),
  );

  const { projects, snapshots, dispose } = await createPersistence();
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
      new GoCorrelationEngine({
        executable: resolve(
          process.env['CHEF_GO_BINARY'] ??
            `artifacts/bin/correlate${process.platform === 'win32' ? '.exe' : ''}`,
        ),
      }),
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
    ...(allowedOAuthUserIds === undefined ? {} : { allowedOAuthUserIds }),
  });

  api.listen(PORT, HOST, () => {
    // No secrets, tokens or scope contents are printed on purpose.
    console.log(`CHEF API listening on http://${HOST}:${PORT}`);
  });

  for (const signal of ['SIGINT', 'SIGTERM'] as const)
    process.on(signal, () => {
      api.close(() => {
        void dispose().finally(() => process.exit(0));
      });
    });
}

main().catch((error: unknown) => {
  console.error(error);
  process.exit(1);
});
