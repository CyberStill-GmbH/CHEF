import { ChefError } from '../../domain/src/errors.js';
import type { Snapshot } from '../../domain/src/model.js';
import type { Membership, Project } from '../../domain/src/project.js';
import type {
  ProjectRepository,
  ProjectSummary,
  SnapshotStore,
} from '../../domain/src/ports.js';

function membershipKey(projectId: string, userId: string): string {
  return `${projectId}\u0000${userId}`;
}

/**
 * Reference adapter for tests and for the offline demo.
 * It enforces the same isolation as the future database adapter, including the
 * "absent membership is not access" rule, so a test that passes here is a real
 * regression test for the port contract.
 */
export class InMemoryProjectRepository implements ProjectRepository {
  private readonly projects = new Map<string, Project>();
  private readonly memberships = new Map<string, Membership>();

  async saveProject(project: Project): Promise<void> {
    if (this.projects.has(project.id))
      throw new ChefError('CONFLICT', 'Project already exists');
    this.projects.set(project.id, { ...project });
  }

  async findProject(id: string): Promise<Project | undefined> {
    const project = this.projects.get(id);
    return project === undefined ? undefined : { ...project };
  }

  async listProjectsForUser(userId: string): Promise<ProjectSummary[]> {
    const summaries: ProjectSummary[] = [];
    for (const membership of this.memberships.values()) {
      if (membership.userId !== userId) continue;
      const project = this.projects.get(membership.projectId);
      if (project === undefined) continue;
      summaries.push({ project: { ...project }, role: membership.role });
    }
    return summaries;
  }

  async saveMembership(membership: Membership): Promise<void> {
    this.memberships.set(
      membershipKey(membership.projectId, membership.userId),
      { ...membership },
    );
  }

  async findMembership(
    projectId: string,
    userId: string,
  ): Promise<Membership | undefined> {
    const membership = this.memberships.get(membershipKey(projectId, userId));
    return membership === undefined ? undefined : { ...membership };
  }
}

export class InMemorySnapshotStore implements SnapshotStore {
  private readonly snapshots = new Map<string, Snapshot>();

  async save(projectId: string, snapshot: Snapshot): Promise<void> {
    this.snapshots.set(projectId, structuredClone(snapshot));
  }

  async latest(projectId: string): Promise<Snapshot | undefined> {
    const snapshot = this.snapshots.get(projectId);
    return snapshot === undefined ? undefined : structuredClone(snapshot);
  }
}
