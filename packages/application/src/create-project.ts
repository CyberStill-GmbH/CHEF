import { ChefError } from '../../domain/src/errors.js';
import {
  normalizeProjectName,
  type Project,
} from '../../domain/src/project.js';
import type {
  ClockPort,
  IdentityPort,
  ProjectRepository,
  RandomPort,
} from '../../domain/src/ports.js';

export interface CreateProjectInput {
  name: string;
  ownerId: string;
}

export class CreateProject {
  constructor(
    private readonly projects: ProjectRepository,
    private readonly identity: IdentityPort,
    private readonly clock: ClockPort,
    private readonly random: RandomPort,
  ) {}

  async execute(input: CreateProjectInput): Promise<Project> {
    if (input.ownerId.trim() === '')
      throw new ChefError('UNAUTHORIZED', 'An authenticated user is required');
    const name = normalizeProjectName(input.name);
    const createdAt = this.clock.now();
    const project: Project = {
      id: this.identity.id('project', [
        input.ownerId,
        name,
        createdAt,
        this.random.token(16),
      ]),
      name,
      ownerId: input.ownerId,
      createdAt,
    };
    await this.projects.saveProject(project);
    await this.projects.saveMembership({
      projectId: project.id,
      userId: input.ownerId,
      role: 'owner',
    });
    return project;
  }
}
