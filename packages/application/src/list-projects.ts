import { ChefError } from '../../domain/src/errors.js';
import type {
  ProjectRepository,
  ProjectSummary,
} from '../../domain/src/ports.js';

export class ListProjects {
  constructor(private readonly projects: ProjectRepository) {}

  async execute(userId: string): Promise<ProjectSummary[]> {
    if (userId.trim() === '')
      throw new ChefError('UNAUTHORIZED', 'An authenticated user is required');
    const summaries = await this.projects.listProjectsForUser(userId);
    return [...summaries].sort((a, b) =>
      a.project.createdAt.localeCompare(b.project.createdAt),
    );
  }
}
