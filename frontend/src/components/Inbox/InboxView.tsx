import React, { useMemo } from 'react';
import { formatDistanceToNow } from 'date-fns';
import * as Select from '@radix-ui/react-select';
import { ICONS } from '../../constants/icons';
import { toast } from 'sonner';
import { PageHeader } from '../Common/PageHeader';
import { SearchField } from '../Common/SearchField';
import { EmptyPanel } from '../Common/EmptyPanel';
import { InlineError } from '../Common/InlineError';
import { useReviewCandidates } from '../../hooks/useReviewCandidates';
import { useTauri } from '../../hooks/useTauri';
import { useAppStore } from '../../store';
import { useRepos } from '../../hooks/useRepos';
import { findSourceRepository } from '../../lib/source-repository';
import type { AnnotatedCandidate, CandidateStatus, ViewType } from '../../types';

interface InboxViewProps {
  onNavigate: (view: ViewType) => void;
}

type InboxFilter = 'awaiting' | 'all' | 'reviewed';

const ALL_REPOSITORIES = '__all__';

function statusLabel(status: CandidateStatus): {
  label: string;
  tone: string;
  canGenerate: boolean;
} {
  if (status === 'new') {
    return { label: 'Not reviewed', tone: 'text-text-secondary', canGenerate: true };
  }
  if ('stale' in status) {
    return { label: 'Updated since review', tone: 'text-status-in_progress', canGenerate: true };
  }
  return { label: 'Reviewed', tone: 'text-status-done', canGenerate: false };
}

export const InboxView: React.FC<InboxViewProps> = ({ onNavigate }) => {
  const {
    candidates,
    errors,
    unsupportedProviders,
    isLoading,
    isFetching,
    error: queryError,
    refetch,
  } = useReviewCandidates();
  const { fetchRemotePr } = useTauri();
  const { data: repos = [] } = useRepos();
  const setSelectedRepoId = useAppStore(state => state.setSelectedRepoId);
  const setParsedDiff = useAppStore(state => state.setParsedDiff);
  const setViewMode = useAppStore(state => state.setViewMode);
  const setDiffText = useAppStore(state => state.setDiffText);
  const setPendingSource = useAppStore(state => state.setPendingSource);
  const setPrRef = useAppStore(state => state.setPrRef);
  const [loadingKey, setLoadingKey] = React.useState<string | null>(null);
  const [filter, setFilter] = React.useState<InboxFilter>('awaiting');
  const [repository, setRepository] = React.useState(ALL_REPOSITORIES);
  const [query, setQuery] = React.useState('');

  const repositories = useMemo(
    () =>
      [...new Set(candidates.map(candidate => candidate.repo))].sort((a, b) => a.localeCompare(b)),
    [candidates]
  );

  const counts = useMemo(() => {
    const awaiting = candidates.filter(
      candidate => statusLabel(candidate.status).canGenerate
    ).length;
    return {
      awaiting,
      all: candidates.length,
      reviewed: candidates.length - awaiting,
    };
  }, [candidates]);

  const filteredCandidates = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();

    return candidates.filter(candidate => {
      const isAwaiting = statusLabel(candidate.status).canGenerate;
      if (filter === 'awaiting' && !isAwaiting) return false;
      if (filter === 'reviewed' && isAwaiting) return false;
      if (repository !== ALL_REPOSITORIES && candidate.repo !== repository) return false;
      if (!normalizedQuery) return true;

      return [candidate.title, candidate.repo, candidate.author, String(candidate.number)].some(
        value => value.toLowerCase().includes(normalizedQuery)
      );
    });
  }, [candidates, filter, query, repository]);

  const grouped = useMemo(() => {
    const actionable = filteredCandidates.filter(c => statusLabel(c.status).canGenerate);
    const done = filteredCandidates.filter(c => !statusLabel(c.status).canGenerate);
    return { actionable, done };
  }, [filteredCandidates]);

  /**
   * Loads the diff and hands off to the Generate view, which owns agent
   * selection and the generation trigger.
   */
  const handleGenerate = async (candidate: AnnotatedCandidate) => {
    if (loadingKey) return;
    const key = `${candidate.providerId}:${candidate.url}`;
    setLoadingKey(key);
    try {
      const parsed = await fetchRemotePr(candidate.url, candidate.providerId);
      setDiffText(parsed.diff_text);
      setPrRef(candidate.url);
      const source = parsed.source ?? candidate.source;
      setPendingSource(source);
      setParsedDiff(parsed);
      setViewMode('diff');
      setSelectedRepoId(findSourceRepository(source, repos)?.id ?? '');
      onNavigate('generate');
    } catch (error) {
      toast('Could not load pull request', {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setLoadingKey(null);
    }
  };

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <PageHeader
        title="Inbox"
        description="Pull requests waiting for your review."
        actions={
          <button
            type="button"
            onClick={() => {
              void refetch();
            }}
            disabled={isFetching}
            aria-busy={isFetching}
            className="bg-bg-secondary text-text-secondary hover:text-text-primary border-border flex h-9 items-center gap-2 rounded-md border px-3 text-xs transition-colors disabled:opacity-50"
          >
            <ICONS.ACTION_REFRESH size={14} className={isFetching ? 'animate-spin' : undefined} />
            {isFetching ? 'Refreshing…' : 'Refresh'}
          </button>
        }
      />

      {!isLoading && candidates.length > 0 && (
        <div className="inbox-toolbar border-border bg-bg-secondary/20 flex flex-wrap items-center gap-3 border-b px-8 py-4">
          <div
            className="border-border bg-bg-secondary flex h-9 shrink-0 items-center rounded-md border p-0.5"
            role="group"
            aria-label="Filter inbox by review status"
          >
            {(['awaiting', 'all', 'reviewed'] as const).map(option => (
              <button
                key={option}
                type="button"
                aria-pressed={filter === option}
                aria-label={`${option[0].toUpperCase()}${option.slice(1)} (${counts[option]})`}
                onClick={() => setFilter(option)}
                className={`h-8 rounded-[4px] px-3 text-xs font-medium transition-[background-color,color,box-shadow] ${
                  filter === option
                    ? 'bg-bg-tertiary text-text-primary shadow-sm'
                    : 'text-text-disabled hover:text-text-secondary'
                }`}
              >
                <span className="capitalize">{option}</span>
                <span className="ml-1.5 font-mono opacity-60">{counts[option]}</span>
              </button>
            ))}
          </div>

          {repositories.length > 1 && (
            <Select.Root value={repository} onValueChange={setRepository}>
              <Select.Trigger
                aria-label="Filter by repository"
                className="border-border bg-bg-secondary text-text-secondary hover:text-text-primary flex h-8 max-w-[240px] min-w-[170px] items-center justify-between gap-2 rounded-md border px-2.5 text-xs transition-[border-color,color] focus:outline-none"
              >
                <span className="flex min-w-0 items-center gap-2">
                  <ICONS.VIEW_REPOS size={13} className="text-text-disabled shrink-0" />
                  <Select.Value />
                </span>
                <Select.Icon className="text-text-disabled shrink-0">
                  <ICONS.CHEVRON_DOWN size={11} />
                </Select.Icon>
              </Select.Trigger>

              <Select.Portal>
                <Select.Content
                  position="popper"
                  sideOffset={6}
                  collisionPadding={12}
                  className="border-border bg-bg-secondary z-50 min-w-[var(--radix-select-trigger-width)] overflow-hidden rounded-lg border shadow-2xl"
                >
                  <Select.Viewport className="p-1">
                    <RepositoryFilterOption value={ALL_REPOSITORIES}>
                      All repositories
                    </RepositoryFilterOption>
                    {repositories.map(repo => (
                      <RepositoryFilterOption key={repo} value={repo}>
                        {repo}
                      </RepositoryFilterOption>
                    ))}
                  </Select.Viewport>
                </Select.Content>
              </Select.Portal>
            </Select.Root>
          )}

          <SearchField
            label="Search inbox"
            clearLabel="Clear inbox search"
            value={query}
            onChange={setQuery}
            placeholder="Title, repository, author, or PR number"
          />
        </div>
      )}

      <div className="product-page-body custom-scrollbar" aria-busy={isFetching}>
        <div className="product-page-content">
          {queryError && (
            <div className="mb-4">
              <InlineError
                title={candidates.length ? 'Could not refresh inbox' : 'Could not load inbox'}
                message={String(queryError)}
                onRetry={() => {
                  void refetch();
                }}
                isRetrying={isFetching}
              />
            </div>
          )}
          {errors.map(err => (
            <div key={err.providerId} className="mb-4">
              <InlineError
                title={`${err.providerName} is unavailable`}
                message={err.message}
                onRetry={() => {
                  void refetch();
                }}
                isRetrying={isFetching}
              />
            </div>
          ))}

          {unsupportedProviders.length > 0 && (
            <div className="border-border/60 bg-bg-secondary/40 text-text-tertiary mb-4 rounded-lg border px-4 py-3 text-xs leading-5">
              {unsupportedProviders.join(', ')} cannot list review requests yet, so nothing from{' '}
              {unsupportedProviders.length === 1 ? 'it' : 'them'} appears here.
            </div>
          )}

          {isLoading && (
            <p className="text-text-secondary text-sm">Checking for review requests…</p>
          )}

          {!isLoading && !queryError && candidates.length === 0 && errors.length === 0 && (
            <EmptyPanel
              icon={<ICONS.VIEW_INBOX size={28} />}
              title="No review requests"
              description="Requests from your connected Git providers will appear here. Refresh to check for new work."
            />
          )}

          {!isLoading && candidates.length > 0 && filteredCandidates.length === 0 && (
            <EmptyPanel
              icon={<ICONS.ACTION_SEARCH size={26} />}
              title={
                query.trim() || repository !== ALL_REPOSITORIES
                  ? 'No matching review requests'
                  : filter === 'reviewed'
                    ? 'No reviewed requests yet'
                    : 'Nothing is waiting on you'
              }
              description={
                query.trim() || repository !== ALL_REPOSITORIES
                  ? 'Try another title, repository, author, or PR number.'
                  : 'You can still browse all requests.'
              }
              action={
                <button
                  type="button"
                  onClick={() => {
                    setFilter('all');
                    setRepository(ALL_REPOSITORIES);
                    setQuery('');
                  }}
                  className="text-text-secondary hover:text-text-primary border-border rounded-md border px-3 py-2 text-xs transition-colors"
                >
                  Show all requests
                </button>
              }
            />
          )}

          {grouped.actionable.length > 0 && (
            <section className="mb-6">
              <h2 className="text-text-secondary mb-3 text-[13px] font-medium">
                Awaiting your review ({grouped.actionable.length})
              </h2>
              <div className="flex flex-col gap-2">
                {grouped.actionable.map(candidate => (
                  <CandidateRow
                    key={`${candidate.providerId}:${candidate.url}`}
                    candidate={candidate}
                    onGenerate={handleGenerate}
                    isLoading={loadingKey === `${candidate.providerId}:${candidate.url}`}
                    disabled={loadingKey !== null}
                  />
                ))}
              </div>
            </section>
          )}

          {grouped.done.length > 0 && (
            <section>
              <h2 className="text-text-secondary mb-3 text-[13px] font-medium">
                Already reviewed ({grouped.done.length})
              </h2>
              <div className="flex flex-col gap-2">
                {grouped.done.map(candidate => (
                  <CandidateRow
                    key={`${candidate.providerId}:${candidate.url}`}
                    candidate={candidate}
                    onGenerate={handleGenerate}
                    isLoading={loadingKey === `${candidate.providerId}:${candidate.url}`}
                    disabled={loadingKey !== null}
                  />
                ))}
              </div>
            </section>
          )}
        </div>
      </div>
    </div>
  );
};

const RepositoryFilterOption: React.FC<React.PropsWithChildren<{ value: string }>> = ({
  value,
  children,
}) => (
  <Select.Item
    value={value}
    className="text-text-primary relative flex cursor-pointer items-center rounded-sm py-2 pr-8 pl-2.5 text-xs outline-none select-none data-[highlighted]:bg-white/10"
  >
    <Select.ItemText>{children}</Select.ItemText>
    <Select.ItemIndicator className="text-brand absolute right-2.5">
      <ICONS.ICON_CHECK size={12} />
    </Select.ItemIndicator>
  </Select.Item>
);

interface CandidateRowProps {
  candidate: AnnotatedCandidate;
  onGenerate: (candidate: AnnotatedCandidate) => void;
  isLoading: boolean;
  disabled: boolean;
}

const CandidateRow: React.FC<CandidateRowProps> = ({
  candidate,
  onGenerate,
  isLoading,
  disabled,
}) => {
  const { openUrl } = useTauri();
  const { label, tone, canGenerate } = statusLabel(candidate.status);
  const updated = new Date(candidate.updatedAt);
  const updatedLabel = Number.isFinite(updated.getTime())
    ? formatDistanceToNow(updated, { addSuffix: true })
    : null;

  return (
    <div className="inbox-request border-border/60 bg-bg-secondary/20 hover:bg-bg-secondary/40 flex items-center gap-4 rounded-lg border px-4 py-4 transition-colors">
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-text-tertiary font-mono text-[11px]">
            {candidate.repo}#{candidate.number}
          </span>
          {candidate.isDraft && (
            <span className="text-text-tertiary border-border rounded border px-1.5 py-0.5 text-[11px]">
              Draft
            </span>
          )}
          <span className={`flex items-center gap-1.5 text-[11px] ${tone}`}>
            <span className="h-1 w-1 rounded-full bg-current" aria-hidden="true" />
            {label}
          </span>
        </div>
        <p
          title={candidate.title}
          className="text-text-primary mt-2 line-clamp-2 text-[15px] leading-6 font-medium"
        >
          {candidate.title}
        </p>
        <p className="text-text-tertiary mt-2 text-xs">
          {candidate.author}
          {updatedLabel && (
            <>
              <span className="mx-2">·</span>
              <time dateTime={candidate.updatedAt} title={updated.toLocaleString()}>
                Updated {updatedLabel}
              </time>
            </>
          )}
        </p>
      </div>

      <div className="inbox-request-actions flex shrink-0 items-center gap-2">
        <button
          onClick={() => openUrl(candidate.url)}
          className="text-text-secondary hover:text-text-primary flex h-8 w-8 shrink-0 items-center justify-center rounded-md transition-colors"
          title="Open in browser"
          aria-label={`Open ${candidate.repo}#${candidate.number} in browser`}
        >
          <ICONS.ACTION_OPEN_WINDOW size={14} />
        </button>

        {canGenerate && (
          <button
            onClick={() => onGenerate(candidate)}
            disabled={disabled}
            aria-busy={isLoading}
            aria-label={`Review ${candidate.repo}#${candidate.number}`}
            className="bg-brand/10 text-brand hover:bg-brand/20 border-brand/20 flex h-8 min-w-20 shrink-0 items-center justify-center rounded-md border px-3 text-xs font-medium transition-colors disabled:opacity-50"
          >
            {isLoading ? 'Loading…' : 'Review'}
          </button>
        )}
      </div>
    </div>
  );
};
