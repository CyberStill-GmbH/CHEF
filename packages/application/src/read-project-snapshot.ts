import { ChefError } from '../../domain/src/errors.js';
import type { Snapshot } from '../../domain/src/model.js';
import { requireMembership } from '../../domain/src/project.js';
import type {
  ProjectRepository,
  SnapshotStore,
} from '../../domain/src/ports.js';

export class ReadProjectSnapshot {
  constructor(
    private readonly projects: ProjectRepository,
    private readonly snapshots: SnapshotStore,
  ) {}

  async execute(projectId: string, userId: string): Promise<Snapshot> {
    requireMembership(
      await this.projects.findMembership(projectId, userId),
      projectId,
    );
    const snapshot = await this.snapshots.latest(projectId);
    if (snapshot === undefined)
      throw new ChefError('NOT_FOUND', 'Project has no imported snapshot');
    return snapshot;
  }
}
