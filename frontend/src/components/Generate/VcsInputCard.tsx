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
  prominent?: boolean;
}

export const VcsInputCard: React.FC<VcsInputCardProps> = ({
  pendingSource,
  prRef,
  onPrRefChange,
  onFetch,
  isLoading,
  disabled,
  onClear,
  prominent = false,
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
      className={`vcs-input-card pointer-events-auto flex max-w-full items-center overflow-hidden border text-xs font-medium transition-[background-color,border-color] duration-150 ${
        prominent ? 'h-11 w-full min-w-0 rounded-md' : 'h-8 w-fit min-w-52 rounded-md'
      } ${
        hasRemoteSource
          ? 'border-border bg-bg-primary text-text-secondary'
          : 'border-border/70 bg-bg-primary text-text-primary'
      }`}
    >
      {hasRemoteSource ? (
        <>
          <div className="border-border flex flex-shrink-0 items-center border-r px-2">
            {vcsSourceIcon}
          </div>
          <span className="max-w-[400px] min-w-0 truncate px-3 font-mono text-xs">
            {vcsSourceLabel}
          </span>
          <div className="flex-1" />
          <div className="border-border flex h-[14px] flex-shrink-0 items-center border-l" />
          <button
            onClick={onClear}
            disabled={disabled || isLoading}
            aria-label="Clear loaded pull request"
            className="flex flex-shrink-0 items-center justify-center px-3 py-1.5 text-[0.625rem] font-medium opacity-80 transition-colors hover:opacity-100"
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
          expanded={prominent}
        />
      )}
    </div>
  );
};
