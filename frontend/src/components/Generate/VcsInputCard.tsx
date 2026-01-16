import { GithubLogo, GitlabLogo, X } from '@phosphor-icons/react';
import type { ReviewSource } from '../../types';
import { PrInput } from './PrInput';

interface VcsInputCardProps {
  pendingSource: ReviewSource | null;
  prRef: string;
  onPrRefChange: (value: string) => void;
  onFetch: () => void;
  isLoading: boolean;
  disabled: boolean;
  onClear: () => void;
}

export const VcsInputCard: React.FC<VcsInputCardProps> = ({
  pendingSource,
  prRef,
  onPrRefChange,
  onFetch,
  isLoading,
  disabled,
  onClear,
}) => {
  const vcsSource =
    pendingSource && (pendingSource.type === 'github_pr' || pendingSource.type === 'gitlab_mr')
      ? pendingSource
      : null;
  const hasRemoteSource = Boolean(vcsSource);
  const vcsSourceLabel = vcsSource
    ? vcsSource.type === 'gitlab_mr'
      ? `${vcsSource.project_path}!${vcsSource.number}`
      : `${vcsSource.owner}/${vcsSource.repo}#${vcsSource.number}`
    : '';
  const vcsSourceIcon = vcsSource ? (
    vcsSource.type === 'gitlab_mr' ? (
      <GitlabLogo size={14} weight="fill" />
    ) : (
      <GithubLogo size={14} weight="fill" />
    )
  ) : null;

  return (
    <div
      className={`ring-border pointer-events-auto flex h-8 w-fit max-w-full min-w-52 items-center overflow-hidden rounded-md border text-xs font-medium backdrop-blur-md transition-colors duration-200 ${
        hasRemoteSource
          ? 'border-green-500/20 bg-green-500/10 text-green-400 shadow-sm'
          : 'border-border bg-bg-secondary/90 text-text-primary shadow-lg shadow-black/20'
      }`}
    >
      {hasRemoteSource ? (
        <>
          <div className="flex flex-shrink-0 items-center border-r border-green-500/20 px-2">
            {vcsSourceIcon}
          </div>
          <span className="max-w-[400px] min-w-0 truncate px-3 font-mono text-xs">
            {vcsSourceLabel}
          </span>
          <div className="flex-1" />
          <div className="flex h-[14px] flex-shrink-0 items-center border-l border-green-500/20" />
          <button
            onClick={onClear}
            className="flex flex-shrink-0 items-center justify-center px-3 py-1.5 text-[10px] font-medium opacity-80 transition-colors hover:opacity-100"
          >
            <X size={12} />
          </button>
        </>
      ) : (
        <PrInput
          prRef={prRef}
          onPrRefChange={onPrRefChange}
          onFetch={onFetch}
          isLoading={isLoading}
          disabled={disabled}
        />
      )}
    </div>
  );
};
