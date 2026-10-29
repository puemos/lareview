import { useState } from 'react';
import { GitBranch, Spinner } from '@phosphor-icons/react';
import type { ReviewSource } from '../../types';

interface LocalGitInputProps {
  repoName?: string;
  source: ReviewSource | null;
  isLoading: boolean;
  disabled: boolean;
  error: string | null;
  onCompare: (base: string, head: string) => void;
  onLinkRepo: () => void;
}

export function LocalGitInput({
  repoName,
  source,
  isLoading,
  disabled,
  error,
  onCompare,
  onLinkRepo,
}: LocalGitInputProps) {
  const [base, setBase] = useState(source?.type === 'local_git' ? source.base_ref : 'main');
  const [head, setHead] = useState(source?.type === 'local_git' ? source.head_ref : 'HEAD');
  const busy = disabled || isLoading;

  return (
    <div className="w-full space-y-3">
      <div className="text-text-secondary flex items-center gap-2 text-xs">
        <GitBranch size={15} />
        {repoName ? (
          <span>Compare refs in {repoName}</span>
        ) : (
          <span>
            Select a repository above or{' '}
            <button type="button" onClick={onLinkRepo} className="text-brand hover:underline">
              link a repository
            </button>
            .
          </span>
        )}
      </div>
      <form
        className="flex flex-wrap items-end gap-2"
        onSubmit={event => {
          event.preventDefault();
          if (repoName && base.trim() && head.trim() && !busy) onCompare(base.trim(), head.trim());
        }}
      >
        <label className="min-w-28 flex-1 space-y-1 text-xs">
          <span className="text-text-tertiary">Base ref</span>
          <input
            aria-label="Base ref"
            value={base}
            onChange={e => setBase(e.target.value)}
            disabled={busy}
            placeholder="main"
            className="border-border bg-bg-secondary text-text-primary focus:border-brand h-9 w-full rounded-md border px-3 font-mono outline-none"
          />
        </label>
        <label className="min-w-28 flex-1 space-y-1 text-xs">
          <span className="text-text-tertiary">Head ref</span>
          <input
            aria-label="Head ref"
            value={head}
            onChange={e => setHead(e.target.value)}
            disabled={busy}
            placeholder="HEAD or feature/branch"
            className="border-border bg-bg-secondary text-text-primary focus:border-brand h-9 w-full rounded-md border px-3 font-mono outline-none"
          />
        </label>
        <button
          type="submit"
          disabled={!repoName || !base.trim() || !head.trim() || busy}
          className="bg-brand hover:bg-brand/90 flex h-9 items-center gap-2 rounded-md px-4 text-xs font-medium text-white disabled:opacity-50"
        >
          {isLoading && <Spinner size={14} className="animate-spin" />}
          {isLoading ? 'Comparing…' : 'Compare refs'}
        </button>
      </form>
      <p className="text-text-disabled text-[11px]">
        Compare two local branch, tag, or commit snapshots. Fetch remote refs with Git first.
      </p>
      {source?.type === 'local_git' && (
        <p className="text-text-secondary font-mono text-[11px]">
          Loaded {source.base_ref} ({source.base_sha.slice(0, 7)}) → {source.head_ref} (
          {source.head_sha.slice(0, 7)})
        </p>
      )}
      {error && (
        <p role="alert" className="text-xs text-red-400">
          {error}
        </p>
      )}
    </div>
  );
}
