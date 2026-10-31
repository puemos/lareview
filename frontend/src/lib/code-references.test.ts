import { describe, expect, it } from 'vitest';
import {
  diffSourceLines,
  resolveCodeReference,
  locationFromHref,
  codeLocationHref,
} from './code-references';
import type { DiffFile } from '../types';

export const file: DiffFile = {
  name: 'src/greeting.ts',
  old_path: 'src/greeting.ts',
  new_path: 'src/greeting.ts',
  hunks: [
    {
      old_start: 10,
      old_lines: 3,
      new_start: 10,
      new_lines: 4,
      content:
        '@@ -10,3 +10,4 @@\n export function greet(name: string) {\n-  return name;\n+  const normalized = name.trim();\n+  return normalized;\n }\n',
    },
  ],
};

describe('evidence code locations', () => {
  it('uses actual old/new line numbers after additions and deletions', () => {
    expect(diffSourceLines(file, 'new').map(row => row.line)).toEqual([10, 11, 12, 13]);
    expect(resolveCodeReference('return normalized;', [file])).toMatchObject({
      line: 12,
      side: 'new',
    });
    expect(resolveCodeReference('return name;', [file])).toMatchObject({ line: 11, side: 'old' });
  });

  it('resolves path/line citations, Markdown anchors, and unique basenames', () => {
    for (const text of [
      'src/greeting.ts:12',
      './src/greeting.ts#L12',
      'src/greeting.ts:12-13',
      'greeting.ts:12',
    ]) {
      expect(resolveCodeReference(text, [file])).toMatchObject({ file, line: 12, side: 'new' });
    }
    const renamed = { ...file, old_path: 'src/old.ts' };
    expect(resolveCodeReference('src/old.ts:11', [renamed])).toMatchObject({
      line: 11,
      side: 'old',
    });
    expect(
      resolveCodeReference('src/greeting.ts:11', [file], {
        file_path: file.name,
        line_number: 11,
        side: 'old',
      })
    ).toMatchObject({ line: 11, side: 'old' });
  });

  it('does not guess when quoted code or filenames are ambiguous', () => {
    const other = {
      ...file,
      name: 'other/greeting.ts',
      new_path: 'other/greeting.ts',
      old_path: 'other/greeting.ts',
    };
    expect(resolveCodeReference('greeting.ts:12', [file, other])).toBeNull();
    expect(resolveCodeReference('return normalized;', [file, other])).toBeNull();
    expect(
      resolveCodeReference('return normalized;', [file, other], {
        file_path: file.name,
        line_number: 12,
        side: 'new',
      })
    ).toMatchObject({ file, line: 12 });
  });

  it('does not link invented code, unknown files, invalid lines, or unsafe URLs', () => {
    for (const text of [
      'src/missing.ts:12',
      'src/greeting.ts:0',
      'src/greeting.ts:99999999999999999',
      'javascript:alert(1)',
      'return sanitize(name);',
    ]) {
      expect(resolveCodeReference(text, [file])).toBeNull();
    }
    expect(locationFromHref('#code/%E0%A4?line=1&side=new', [file])).toBeNull();
  });

  it('matches multiline code only across consecutive lines in the same hunk', () => {
    expect(
      resolveCodeReference('const normalized = name.trim();\nreturn normalized;', [file])
    ).toMatchObject({ line: 11, side: 'new' });
    const separated = {
      ...file,
      hunks: [
        { ...file.hunks[0], content: '+one();', new_lines: 1 },
        { old_start: 50, old_lines: 0, new_start: 50, new_lines: 1, content: '+two();' },
      ],
    };
    expect(resolveCodeReference('one();\ntwo();', [separated])).toBeNull();
    const location = { file, line: 12, side: 'new' as const };
    expect(locationFromHref(codeLocationHref(location), [file])).toEqual(location);
  });

  it('treats percent signs in quoted source as code rather than broken URL encoding', () => {
    const modulo = {
      ...file,
      hunks: [{ ...file.hunks[0], content: '-old();\n+const remainder = count % 2;' }],
    };
    expect(resolveCodeReference('count % 2', [modulo])).toMatchObject({ line: 10, side: 'new' });
  });

  it('does not prefer a new-side match merely because optional path fields are absent', () => {
    const oldMatch: DiffFile = {
      name: 'one.ts',
      new_path: 'one.ts',
      hunks: [
        {
          old_start: 1,
          old_lines: 1,
          new_start: 1,
          new_lines: 1,
          content: '-same();\n+different();',
        },
      ],
    };
    const newMatch: DiffFile = {
      name: 'two.ts',
      new_path: 'two.ts',
      hunks: [
        {
          old_start: 1,
          old_lines: 1,
          new_start: 1,
          new_lines: 1,
          content: '-different();\n+same();',
        },
      ],
    };
    expect(resolveCodeReference('same();', [oldMatch, newMatch])).toBeNull();
  });
});
