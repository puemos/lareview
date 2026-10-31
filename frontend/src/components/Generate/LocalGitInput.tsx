import { useId, useState } from 'react';
import { GitBranch, ArrowRight, Spinner, WarningCircle } from '@phosphor-icons/react';
import type { ReviewSource } from '../../types';

interface LocalGitInputProps {
  repoName?: string;
  source: ReviewSource | null;
  isLoading: boolean;
  disabled: boolean;
  compact?: boolean;
  error: string | null;
  onCompare: (base: string, head: string) => Promise<boolean>;
  onLinkRepo: () => void;
  onCancelEdit?: () => void;
}

export function LocalGitInput({
  repoName,
  source,
  isLoading,
  disabled,
  compact = false,
  error,
  onCompare,
  onLinkRepo,
  onCancelEdit,
}: LocalGitInputProps) {
  const [base, setBase] = useState(source?.type === 'local_git' ? source.base_ref : 'main');
  const [head, setHead] = useState(source?.type === 'local_git' ? source.head_ref : 'HEAD');
  const inputId = useId();
  const helperId = `${inputId}-help`;
  const errorId = `${inputId}-error`;
  const busy = disabled || isLoading;
  const [editing, setEditing] = useState(false);

  if (compact && source?.type === 'local_git' && !editing) {
    return (
      <div className="local-source-summary flex min-w-0 items-center gap-3">
        <GitBranch size={15} className="text-text-tertiary shrink-0" />
        <span className="local-source-repo text-text-primary max-w-40 truncate text-xs font-medium">
          {repoName}
        </span>
        <span className="text-text-disabled">/</span>
        <span
          className="local-source-refs text-text-secondary flex min-w-0 items-center gap-2 font-mono text-[0.6875rem]"
          title={`${source.base_sha} → ${source.head_sha}`}
        >
          <span className="max-w-36 truncate">{source.base_ref}</span>
          <ArrowRight size={12} className="shrink-0" />
          <span className="max-w-36 truncate">{source.head_ref}</span>
        </span>
        <span className="local-source-ready text-status-done ml-1 text-[0.625rem]">Ready</span>
        <button
          type="button"
          disabled={busy}
          onClick={() => setEditing(true)}
          className="text-text-tertiary hover:text-text-primary rounded px-2 py-1 text-[0.6875rem] transition-colors"
          aria-label="Edit comparison"
        >
          Change
        </button>
      </div>
    );
  }

  return (
    <div className="local-git-input w-full space-y-4">
      <div className="text-text-secondary flex items-center gap-2 text-xs">
        <GitBranch size={15} />
        {repoName ? (
          <span className="text-text-primary font-medium">{repoName}</span>
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
        className="local-git-form grid gap-4"
        onSubmit={async event => {
          event.preventDefault();
          if (repoName && base.trim() && head.trim() && !busy) {
            const loaded = await onCompare(base.trim(), head.trim());
            if (loaded) setEditing(false);
          }
        }}
      >
        <div className="local-ref-pair">
          <label className="local-ref-field">
            <span>Base</span>
            <input
              aria-label="Base ref"
              aria-invalid={Boolean(error)}
              aria-describedby={error ? errorId : helperId}
              value={base}
              onChange={e => setBase(e.target.value)}
              disabled={busy}
              placeholder="main"
              className="text-text-primary w-full min-w-0 bg-transparent font-mono text-[0.8125rem] outline-none"
            />
          </label>
          <ArrowRight size={15} className="text-text-disabled self-center" />
          <label className="local-ref-field">
            <span>Head</span>
            <input
              aria-label="Head ref"
              aria-invalid={Boolean(error)}
              aria-describedby={error ? errorId : helperId}
              value={head}
              onChange={e => setHead(e.target.value)}
              disabled={busy}
              placeholder="HEAD or feature/branch"
              className="text-text-primary w-full min-w-0 bg-transparent font-mono text-[0.8125rem] outline-none"
            />
          </label>
        </div>
        <div className="flex items-center justify-between gap-4">
          <p id={helperId} className="text-text-disabled text-[0.6875rem]">
            Branches, tags, or commit hashes.
          </p>
          <div className="flex items-center gap-2">
            {compact && editing && (
              <button
                type="button"
                className="text-text-secondary hover:text-text-primary rounded px-3 py-2 text-xs"
                disabled={busy}
                onClick={() => {
                  if (source?.type === 'local_git') {
                    setBase(source.base_ref);
                    setHead(source.head_ref);
                  }
                  setEditing(false);
                  onCancelEdit?.();
                }}
              >
                Cancel
              </button>
            )}
            <button
              type="submit"
              aria-busy={isLoading}
              disabled={!repoName || !base.trim() || !head.trim() || busy}
              className="generate-primary-button flex h-9 min-w-[152px] shrink-0 items-center justify-center gap-2 rounded-md px-4 text-xs font-medium whitespace-nowrap"
            >
              {isLoading && <Spinner size={14} className="animate-spin" />}
              {isLoading ? 'Comparing…' : 'Compare changes'}
              {!isLoading && <ArrowRight size={13} />}
            </button>
          </div>
        </div>
      </form>
      {source?.type === 'local_git' && (
        <p className="text-text-secondary font-mono text-[0.6875rem]">
          Loaded {source.base_ref} ({source.base_sha.slice(0, 7)}) → {source.head_ref} (
          {source.head_sha.slice(0, 7)})
        </p>
      )}
      {error && (
        <div
          id={errorId}
          role="alert"
          className="flex items-start gap-2 rounded-md border border-red-400/15 bg-red-400/5 px-3 py-2.5 text-xs leading-5 text-red-400"
        >
          <WarningCircle size={15} className="mt-0.5 shrink-0" />
          <p>{error}</p>
        </div>
      )}
    </div>
  );
}
