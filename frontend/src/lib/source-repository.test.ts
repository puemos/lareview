import { describe, expect, it } from 'vitest';
import { findSourceRepository } from './source-repository';

const source = { type: 'github_pr' as const, owner: 'team', repo: 'project', number: 1 };

describe('source repository matching', () => {
  it('matches exact HTTPS and SCP remote identities', () => {
    for (const remote of [
      'https://github.com/team/project.git',
      'git@github.com:Team/Project.git',
    ]) {
      const repo = { id: 'correct', remotes: [remote] };
      expect(findSourceRepository(source, [repo])).toBe(repo);
    }
  });
  it('does not select a matching basename, path prefix, or different host', () => {
    expect(
      findSourceRepository(source, [
        { name: 'project', remotes: ['https://github.com/team/project-old.git'] },
        { name: 'project', remotes: ['https://gitlab.com/team/project.git'] },
        { name: 'project', remotes: [] },
      ])
    ).toBeUndefined();
  });
  it('matches GitLab namespace paths and SSH URLs', () => {
    const repo = { remotes: ['ssh://git@code.internal/group/sub/project.git'] };
    expect(
      findSourceRepository(
        { type: 'gitlab_mr', host: 'code.internal', project_path: 'group/sub/project', number: 3 },
        [repo]
      )
    ).toBe(repo);
  });
});
