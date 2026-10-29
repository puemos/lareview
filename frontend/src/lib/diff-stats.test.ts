import { describe, expect, it } from 'vitest';
import { countDiffChanges } from './diff-stats';

describe('countDiffChanges', () => {
  it('excludes context and counts added/deleted lines even when code begins with a plus or minus', () => {
    expect(
      countDiffChanges([
        {
          name: 'a.ts',
          new_path: 'a.ts',
          hunks: [
            {
              old_start: 1,
              old_lines: 5,
              new_start: 1,
              new_lines: 6,
              content:
                '@@ -1,5 +1,6 @@\n context\n-old\n+new\n+++counter;\n\\ No newline at end of file\n',
            },
          ],
        },
      ])
    ).toEqual({ additions: 2, deletions: 1 });
  });
});
