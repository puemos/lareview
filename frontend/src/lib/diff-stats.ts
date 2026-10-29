import type { DiffFile } from '../types';

/** Hunk ranges include context; only prefixed content lines are changes. */
export function countDiffChanges(files: DiffFile[]) {
  let additions = 0;
  let deletions = 0;
  for (const file of files) {
    for (const hunk of file.hunks) {
      for (const line of (hunk.content ?? '').split('\n')) {
        if (line.startsWith('+')) additions++;
        else if (line.startsWith('-')) deletions++;
      }
    }
  }
  return { additions, deletions };
}
