import type { ReviewSource } from '../types';

function remoteIdentity(remote: string): string | null {
  try {
    const url = new URL(remote);
    return `${url.host}/${url.pathname.replace(/^\/+|\/+$/g, '').replace(/\.git$/i, '')}`.toLowerCase();
  } catch {
    const scp = remote.match(/^(?:[^@/:]+@)?([^/:]+):(.+)$/);
    return scp
      ? `${scp[1]}/${scp[2].replace(/^\/+|\/+$/g, '').replace(/\.git$/i, '')}`.toLowerCase()
      : null;
  }
}

export function findSourceRepository<T extends { remotes: string[] }>(
  source: ReviewSource | null | undefined,
  repositories: T[]
): T | undefined {
  const identity =
    source?.type === 'github_pr'
      ? `github.com/${source.owner}/${source.repo}`
      : source?.type === 'gitlab_mr'
        ? `${source.host}/${source.project_path}`
        : null;
  if (!identity) return undefined;
  return repositories.find(repository =>
    repository.remotes.some(remote => remoteIdentity(remote) === identity.toLowerCase())
  );
}
