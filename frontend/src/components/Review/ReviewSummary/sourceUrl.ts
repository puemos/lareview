import type { ReviewSource } from '../../../types';

/**
 * Canonical web URL for a review source. Older reviews were stored without one,
 * so it is reconstructed from the identifying parts when absent.
 */
export function sourceUrl(source: ReviewSource): string | null {
  if (source.type === 'github_pr') {
    return source.url ?? `https://github.com/${source.owner}/${source.repo}/pull/${source.number}`;
  }
  if (source.type === 'gitlab_mr') {
    return (
      source.url ??
      `https://${source.host}/${source.project_path}/-/merge_requests/${source.number}`
    );
  }
  return null;
}
