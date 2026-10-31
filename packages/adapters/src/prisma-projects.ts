import { ChefError } from '../../domain/src/errors.js';
import type { Snapshot } from '../../domain/src/model.js';
import type { Membership, Project, Role } from '../../domain/src/project.js';
import type {
  ClockPort,
  ProjectRepository,
  ProjectSummary,
  SnapshotStore,
} from '../../domain/src/ports.js';

/**
 * Adaptador para el cliente generado por Prisma.
 *
 * No importa `@prisma/client`: describe solo la forma de los delegados que el
 * generador produce, para poder probar el mapeo con un cliente falso y compilar
 * sin generar el cliente. Al componer la API se pasa la instancia real.
 *
 * Probado con cliente falso y con PostgreSQL 16 temporal mediante db:verify:
 * dos usuarios/proyectos, persistencia al reiniciar y recuperación desde backup.
 * La validación del entorno remoto sigue siendo un paso del despliegue.
 */

export interface PrismaProjectRow {
  readonly id: string;
  readonly name: string;
  readonly ownerId: string;
  readonly createdAt: Date;
}

export interface PrismaMembershipRow {
  readonly projectId: string;
  readonly userId: string;
  readonly role: string;
  readonly project?: PrismaProjectRow;
}

export interface PrismaSnapshotRow {
  readonly payload: unknown;
}

export interface PrismaProjectDelegate {
  create(args: {
    data: {
      id: string;
      name: string;
      ownerId: string;
      createdAt: Date;
    };
  }): Promise<PrismaProjectRow>;
  findUnique(args: { where: { id: string } }): Promise<PrismaProjectRow | null>;
}

export interface PrismaMembershipDelegate {
  upsert(args: {
    where: { projectId_userId: { projectId: string; userId: string } };
    create: { projectId: string; userId: string; role: Role };
    update: { role: Role };
  }): Promise<unknown>;
  findUnique(args: {
    where: { projectId_userId: { projectId: string; userId: string } };
  }): Promise<PrismaMembershipRow | null>;
  findMany(args: {
    where: { userId: string };
    include: { project: true };
  }): Promise<PrismaMembershipRow[]>;
}

export interface PrismaSnapshotDelegate {
  create(args: {
    data: {
      projectId: string;
      runId: string;
      payload: unknown;
      createdAt: Date;
    };
  }): Promise<unknown>;
  findFirst(args: {
    where: { projectId: string };
    orderBy: { createdAt: 'desc' };
  }): Promise<PrismaSnapshotRow | null>;
}

export interface PrismaLikeClient {
  readonly project: PrismaProjectDelegate;
  readonly membership: PrismaMembershipDelegate;
  readonly snapshot: PrismaSnapshotDelegate;
}

function toProject(row: PrismaProjectRow): Project {
  return {
    id: row.id,
    name: row.name,
    ownerId: row.ownerId,
    createdAt: row.createdAt.toISOString(),
  };
}

function toRole(value: string): Role {
  if (value === 'owner' || value === 'member') return value;
  throw new Error('Stored membership has an unknown role');
}

function isUniqueViolation(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    (error as { code?: unknown }).code === 'P2002'
  );
}

function isSnapshotPayload(value: unknown): value is Snapshot {
  if (typeof value !== 'object' || value === null) return false;
  const record = value as Record<string, unknown>;
  return (
    record['schemaVersion'] === '1.0.0' &&
    Array.isArray(record['observations']) &&
    Array.isArray(record['assets'])
  );
}

export class PrismaProjectRepository implements ProjectRepository {
  constructor(private readonly client: PrismaLikeClient) {}

  async saveProject(project: Project): Promise<void> {
    try {
      await this.client.project.create({
        data: {
          id: project.id,
          name: project.name,
          ownerId: project.ownerId,
          createdAt: new Date(project.createdAt),
        },
      });
    } catch (error) {
      // Una PK duplicada es un conflicto de dominio, no un 500.
      if (isUniqueViolation(error))
        throw new ChefError('CONFLICT', 'Project already exists');
      throw error;
    }
  }

  async findProject(id: string): Promise<Project | undefined> {
    const row = await this.client.project.findUnique({ where: { id } });
    return row === null ? undefined : toProject(row);
  }

  async listProjectsForUser(userId: string): Promise<ProjectSummary[]> {
    const rows = await this.client.membership.findMany({
      where: { userId },
      include: { project: true },
    });
    const summaries: ProjectSummary[] = [];
    for (const row of rows) {
      if (row.project === undefined) continue;
      summaries.push({
        project: toProject(row.project),
        role: toRole(row.role),
      });
    }
    return summaries;
  }

  async saveMembership(membership: Membership): Promise<void> {
    await this.client.membership.upsert({
      where: {
        projectId_userId: {
          projectId: membership.projectId,
          userId: membership.userId,
        },
      },
      create: {
        projectId: membership.projectId,
        userId: membership.userId,
        role: membership.role,
      },
      update: { role: membership.role },
    });
  }

  async findMembership(
    projectId: string,
    userId: string,
  ): Promise<Membership | undefined> {
    const row = await this.client.membership.findUnique({
      where: { projectId_userId: { projectId, userId } },
    });
    if (row === null) return undefined;
    return {
      projectId: row.projectId,
      userId: row.userId,
      role: toRole(row.role),
    };
  }
}

export class PrismaSnapshotStore implements SnapshotStore {
  constructor(
    private readonly client: PrismaLikeClient,
    private readonly clock: ClockPort,
  ) {}

  async save(projectId: string, snapshot: Snapshot): Promise<void> {
    await this.client.snapshot.create({
      data: {
        projectId,
        runId: snapshot.run.id,
        payload: snapshot,
        createdAt: new Date(this.clock.now()),
      },
    });
  }

  async latest(projectId: string): Promise<Snapshot | undefined> {
    const row = await this.client.snapshot.findFirst({
      where: { projectId },
      orderBy: { createdAt: 'desc' },
    });
    if (row === null) return undefined;
    // Un payload corrupto no debe llegar a la UI como si fuera un snapshot.
    if (!isSnapshotPayload(row.payload))
      throw new Error('Stored snapshot does not match Snapshot 1.0.0');
    return structuredClone(row.payload);
  }
}
