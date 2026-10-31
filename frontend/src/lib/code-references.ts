import type { DiffFile, FeedbackAnchor } from '../types';

export interface CodeLocation {
  file: DiffFile;
  line: number;
  side: 'old' | 'new';
}

export function diffSourceLines(file: DiffFile, side: 'old' | 'new') {
  return file.hunks.flatMap(hunk => {
    let line = side === 'old' ? hunk.old_start : hunk.new_start;
    return (hunk.content ?? '').split('\n').flatMap(text => {
      if (!/^[ +-]/.test(text)) return [];
      if (side === 'old' ? text.startsWith('+') : text.startsWith('-')) return [];
      return [{ line: line++, content: text.slice(1) }];
    });
  });
}

export function resolveCodeReference(
  text: string,
  files: DiffFile[],
  anchor?: FeedbackAnchor | null
): CodeLocation | null {
  const reference = text.trim();
  const path = /^(.*?)(?::([1-9]\d*)(?::\d+|-[1-9]\d*)?|#L([1-9]\d*)(?:-L?[1-9]\d*)?)?$/.exec(
    reference
  );
  if (path) {
    let name: string;
    try {
      name = decodeURIComponent(path[1]).replace(/^\.\//, '');
    } catch {
      name = path[1].replace(/^\.\//, '');
    }
    const exactFile = files.find(item => [item.name, item.new_path, item.old_path].includes(name));
    const basenameMatches =
      !name.includes('/') && name.includes('.')
        ? files.filter(item => item.name.split('/').pop() === name)
        : [];
    const file = exactFile ?? (basenameMatches.length === 1 ? basenameMatches[0] : undefined);
    if (file) {
      const side =
        name === file.old_path && file.old_path !== file.new_path
          ? 'old'
          : anchor?.file_path &&
              [file.name, file.new_path, file.old_path].includes(anchor.file_path)
            ? (anchor?.side ?? 'new')
            : 'new';
      const line = Number(path[2] ?? path[3] ?? diffSourceLines(file, side)[0]?.line ?? 1);
      return Number.isSafeInteger(line) && line > 0 ? { file, line, side } : null;
    }
  }

  // Link quoted code only when it can be located in the reviewed source.
  if (reference.length < 3) return null;
  const quotedLines = reference.split('\n').map(line => line.trim());
  const literalPattern =
    quotedLines.length === 1
      ? new RegExp(`(?:^|[^\\w$])${quotedLines[0].replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(?![\\w$])`)
      : null;
  const matches: CodeLocation[] = [];
  for (const file of files) {
    for (const side of ['new', 'old'] as const) {
      const lines = diffSourceLines(file, side);
      for (let index = 0; index < lines.length; index++) {
        const matchesQuote = quotedLines.every((quote, offset) => {
          const current = lines[index + offset];
          if (!current || current.line !== lines[index].line + offset) return false;
          if (quotedLines.length > 1) return current.content.trim() === quote;
          return literalPattern!.test(current.content);
        });
        if (matchesQuote) matches.push({ file, line: lines[index].line, side });
      }
    }
  }
  const preferred = matches.filter(
    match =>
      anchor?.file_path &&
      [match.file.name, match.file.new_path, match.file.old_path].includes(anchor.file_path) &&
      match.side === (anchor?.side ?? 'new')
  );
  const exactAnchor = preferred.find(match => match.line === anchor?.line_number);
  if (exactAnchor) return exactAnchor;
  if (preferred.length === 1) return preferred[0];
  // An unchanged line exists on both sides; prefer its new-side location.
  const unique = matches.filter(
    match =>
      match.side === 'new' ||
      !matches.some(
        other => other.file === match.file && other.side === 'new' && other.line === match.line
      )
  );
  return unique.length === 1 ? unique[0] : null;
}

export function codeLocationHref(location: CodeLocation) {
  return `#code/${encodeURIComponent(location.file.name)}?line=${location.line}&side=${location.side}`;
}

export function locationFromHref(
  href: string,
  files: DiffFile[],
  anchor?: FeedbackAnchor | null
): CodeLocation | null {
  const match = /^#code\/([^?]+)\?line=([1-9]\d*)&side=(old|new)$/.exec(href);
  if (!match) return resolveCodeReference(href, files, anchor);
  try {
    const file = files.find(file => file.name === decodeURIComponent(match[1]));
    const line = Number(match[2]);
    return file && Number.isSafeInteger(line)
      ? { file, line, side: match[3] as 'old' | 'new' }
      : null;
  } catch {
    return null;
  }
}

interface MarkdownNode {
  type: string;
  value?: string;
  children?: MarkdownNode[];
  url?: string;
}

export function codeReferencePlugin(files: DiffFile[], anchor?: FeedbackAnchor | null) {
  const paths = [
    ...new Set(files.flatMap(file => [file.name, file.new_path, file.old_path]).filter(Boolean)),
  ];
  const basenames = new Map<string, Set<DiffFile>>();
  for (const file of files) {
    const basename = file.name.split('/').pop()!;
    basenames.set(basename, new Set([...(basenames.get(basename) ?? []), file]));
  }
  for (const [basename, matches] of basenames) {
    if (matches.size === 1 && basename.includes('.')) paths.push(basename);
  }
  if (!paths.length) return () => {};
  const escaped = paths
    .map(path => path!.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
    .sort((a, b) => b.length - a.length);
  const pattern = new RegExp(
    `(?:\\./)?(?:${escaped.join('|')})(?::[1-9]\\d*(?::\\d+|-[1-9]\\d*)?|#L[1-9]\\d*(?:-L?[1-9]\\d*)?)?(?![\\w/])`,
    'g'
  );
  return () => (tree: MarkdownNode) => {
    const visit = (node: MarkdownNode) => {
      if (['link', 'image', 'code', 'inlineCode', 'html'].includes(node.type)) return;
      node.children = node.children?.flatMap(child => {
        if (child.type === 'inlineCode' && child.value) {
          const location = resolveCodeReference(child.value, files, anchor);
          return [
            location ? { type: 'link', url: codeLocationHref(location), children: [child] } : child,
          ];
        }
        if (child.type !== 'text' || !child.value) {
          visit(child);
          return [child];
        }
        const result: MarkdownNode[] = [];
        let offset = 0;
        for (const match of child.value.matchAll(pattern)) {
          if (match.index! > 0 && /[\w/]/.test(child.value[match.index! - 1])) continue;
          const location = resolveCodeReference(match[0], files, anchor);
          if (!location) continue;
          result.push({ type: 'text', value: child.value.slice(offset, match.index) });
          result.push({
            type: 'link',
            url: codeLocationHref(location),
            children: [{ type: 'text', value: match[0] }],
          });
          offset = match.index! + match[0].length;
        }
        result.push({ type: 'text', value: child.value.slice(offset) });
        return result;
      });
    };
    visit(tree);
  };
}
