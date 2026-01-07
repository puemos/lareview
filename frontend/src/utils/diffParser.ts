import type { ParsedDiff, DiffFile, DiffHunk } from '../types';

export function parseDiffLocally(diffText: string): ParsedDiff {
  const files: DiffFile[] = [];
  const lines = diffText.split('\n');
  let currentFile: DiffFile | null = null;
  let currentHunk: DiffHunk | null = null;

  let totalAdditions = 0;
  let totalDeletions = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    if (line.startsWith('diff --git')) {
      if (currentFile) {
        files.push(currentFile);
      }
      const match = line.match(/diff --git a\/(.*) b\/(.*)/);
      const path = match ? match[2] : 'unknown';
      currentFile = {
        name: path,
        new_path: path,
        hunks: [],
        status: 'modified',
      };
      currentHunk = null;
    } else if (line.startsWith('--- ')) {
      if (currentFile) currentFile.old_path = line.slice(4).replace(/^a\//, '');
    } else if (line.startsWith('+++ ')) {
      if (currentFile) {
        currentFile.new_path = line.slice(4).replace(/^b\//, '');
        currentFile.name = currentFile.new_path;
      }
    } else if (line.startsWith('@@ ')) {
      if (currentFile) {
        const match = line.match(/@@ -(\d+),?(\d*) \+(\d+),?(\d*) @@/);
        if (match) {
          currentHunk = {
            old_start: parseInt(match[1]),
            old_lines: parseInt(match[2] || '1'),
            new_start: parseInt(match[3]),
            new_lines: parseInt(match[4] || '1'),
            content: line + '\n',
            header: line,
          };
          currentFile.hunks.push(currentHunk);
        }
      }
    } else if (currentHunk) {
      currentHunk.content += line + '\n';
      if (line.startsWith('+')) totalAdditions++;
      else if (line.startsWith('-')) totalDeletions++;
    }
  }

  if (currentFile) {
    files.push(currentFile);
  }

  // If no "diff --git" headers found, try to treat as a single file diff if headers like ---/+++ exist
  if (files.length === 0 && (diffText.includes('--- ') || diffText.includes('+++ '))) {
    const fileNameMatch = diffText.match(/\+\+\+ b\/(.*)/) || diffText.match(/\+\+\+ (.*)/);
    const path = fileNameMatch ? fileNameMatch[1] : 'pasted_diff';

    currentFile = {
      name: path,
      new_path: path,
      hunks: [],
    };

    let currentHunk: DiffHunk | null = null;
    lines.forEach(line => {
      if (line.startsWith('@@ ')) {
        const match = line.match(/@@ -(\d+),?(\d*) \+(\d+),?(\d*) @@/);
        if (match) {
          currentHunk = {
            old_start: parseInt(match[1]),
            old_lines: parseInt(match[2] || '1'),
            new_start: parseInt(match[3]),
            new_lines: parseInt(match[4] || '1'),
            content: line + '\n',
            header: line,
          };
          currentFile!.hunks.push(currentHunk);
        }
      } else if (currentHunk) {
        currentHunk.content += line + '\n';
        if (line.startsWith('+')) totalAdditions++;
        else if (line.startsWith('-')) totalDeletions++;
      }
    });
    files.push(currentFile);
  }

  return {
    diff_text: diffText,
    files,
    total_additions: totalAdditions,
    total_deletions: totalDeletions,
    changed_files: files.length,
  };
}
