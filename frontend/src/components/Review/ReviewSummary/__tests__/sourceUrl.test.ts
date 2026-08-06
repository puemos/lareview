import { describe, it, expect } from 'vitest';
import { sourceUrl } from '../sourceUrl';

describe('sourceUrl', () => {
  it('uses the stored url when present', () => {
    expect(
      sourceUrl({
        type: 'github_pr',
        owner: 'acme',
        repo: 'widget',
        number: 7,
        url: 'https://github.enterprise.internal/acme/widget/pull/7',
      })
    ).toBe('https://github.enterprise.internal/acme/widget/pull/7');
  });

  it('reconstructs a GitHub url when none was stored', () => {
    expect(sourceUrl({ type: 'github_pr', owner: 'acme', repo: 'widget', number: 7 })).toBe(
      'https://github.com/acme/widget/pull/7'
    );
  });

  it('reconstructs a GitLab url using the recorded host', () => {
    expect(
      sourceUrl({
        type: 'gitlab_mr',
        host: 'gitlab.example.com',
        project_path: 'group/proj',
        number: 12,
      })
    ).toBe('https://gitlab.example.com/group/proj/-/merge_requests/12');
  });

  it('has no url for a pasted diff', () => {
    expect(sourceUrl({ type: 'diff_paste', diff_hash: 'abc' })).toBeNull();
  });
});
