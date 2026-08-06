import React, { useMemo } from 'react';
import { formatDistanceToNow } from 'date-fns';
import * as Select from '@radix-ui/react-select';
import { ICONS } from '../../constants/icons';
import { toast } from 'sonner';
import { useReviewCandidates } from '../../hooks/useReviewCandidates';
import { useTauri } from '../../hooks/useTauri';
import { useAppStore } from '../../store';
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
    return { label: 'Moved on since review', tone: 'text-status-in_progress', canGenerate: true };
  }
  return { label: 'Reviewed', tone: 'text-status-done', canGenerate: false };
}

export const InboxView: React.FC<InboxViewProps> = ({ onNavigate }) => {
  const { candidates, errors, unsupportedProviders, isLoading, isFetching, refetch } =
    useReviewCandidates();
  const { fetchRemotePr } = useTauri();
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
    const key = `${candidate.repo}#${candidate.number}`;
    setLoadingKey(key);
    try {
      const parsed = await fetchRemotePr(candidate.url, candidate.providerId);
      setDiffText(parsed.diff_text);
      setPrRef(candidate.url);
      if (parsed.source) setPendingSource(parsed.source);
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
      <header className="border-border flex items-center justify-between border-b px-6 py-4">
        <div>
          <h1 className="text-text-primary text-lg font-semibold">Inbox</h1>
          <p className="text-text-secondary text-xs">
            Pull requests where you have been asked to review
          </p>
        </div>
        <button
          onClick={() => refetch()}
          disabled={isFetching}
          className="bg-bg-tertiary hover:bg-bg-secondary text-text-secondary hover:text-text-primary border-border flex items-center gap-1.5 rounded border px-2.5 py-1.5 text-xs transition-colors disabled:opacity-50"
        >
          <ICONS.ACTION_REFRESH size={14} className={isFetching ? 'animate-spin' : undefined} />
          Refresh
        </button>
      </header>

      {!isLoading && candidates.length > 0 && (
        <div className="border-border bg-bg-secondary/20 flex flex-wrap items-center gap-3 border-b px-6 py-3">
          <div
            className="border-border bg-bg-secondary flex h-8 shrink-0 items-center rounded-md border p-0.5"
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
                className={`h-7 rounded-[4px] px-2.5 text-[10px] font-medium transition-[background-color,color,box-shadow] ${
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

          <div className="relative max-w-sm min-w-[220px] flex-1">
            <ICONS.ACTION_SEARCH
              size={13}
              className="text-text-disabled pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2"
            />
            <input
              type="search"
              value={query}
              onChange={event => setQuery(event.target.value)}
              placeholder="Filter by title, repository, author, or PR number"
              aria-label="Search inbox"
              className="bg-bg-tertiary border-border text-text-primary placeholder:text-text-disabled focus:border-brand focus:ring-brand/20 h-8 w-full rounded-md border pr-8 pl-8 text-xs transition-[border-color,box-shadow] focus:ring-1 focus:outline-none"
            />
            {query && (
              <button
                type="button"
                onClick={() => setQuery('')}
                aria-label="Clear inbox search"
                className="text-text-disabled hover:text-text-primary absolute top-1/2 right-2 flex h-5 w-5 -translate-y-1/2 items-center justify-center rounded transition-colors"
              >
                <ICONS.ACTION_CLOSE size={11} />
              </button>
            )}
          </div>
        </div>
      )}

      <div className="custom-scrollbar flex-1 overflow-y-auto px-6 py-4">
        {errors.map(err => (
          <div
            key={err.providerId}
            className="border-status-error/20 bg-status-error/5 text-status-error mb-3 rounded-lg border px-3 py-2 text-xs"
          >
            <span className="font-medium">{err.providerName}</span> could not be reached:{' '}
            {err.message}
          </div>
        ))}

        {unsupportedProviders.length > 0 && (
          <div className="border-border bg-bg-secondary/40 text-text-secondary mb-3 rounded-lg border px-3 py-2 text-xs">
            {unsupportedProviders.join(', ')} cannot list review requests yet, so nothing from{' '}
            {unsupportedProviders.length === 1 ? 'it' : 'them'} appears here.
          </div>
        )}

        {isLoading && <p className="text-text-secondary text-sm">Checking for review requests…</p>}

        {!isLoading && candidates.length === 0 && errors.length === 0 && (
          <div className="text-text-secondary flex flex-col items-center justify-center py-16 text-center">
            <ICONS.STATUS_SUCCESS size={40} className="mb-3 opacity-40" />
            <p className="text-sm">Nothing is waiting on you.</p>
          </div>
        )}

        {!isLoading && candidates.length > 0 && filteredCandidates.length === 0 && (
          <div className="text-text-secondary flex flex-col items-center justify-center py-16 text-center">
            <ICONS.ACTION_SEARCH size={32} className="mb-3 opacity-30" />
            <p className="text-sm">No review requests match these filters.</p>
            <button
              type="button"
              onClick={() => {
                setFilter('all');
                setRepository(ALL_REPOSITORIES);
                setQuery('');
              }}
              className="text-brand hover:text-brand/80 mt-2 text-xs font-medium transition-colors"
            >
              Show all requests
            </button>
          </div>
        )}

        {grouped.actionable.length > 0 && (
          <section className="mb-6">
            <h2 className="text-text-secondary mb-2 text-xs font-bold tracking-wider uppercase">
              Awaiting your review ({grouped.actionable.length})
            </h2>
            <div className="flex flex-col gap-2">
              {grouped.actionable.map(candidate => (
                <CandidateRow
                  key={`${candidate.repo}#${candidate.number}`}
                  candidate={candidate}
                  onGenerate={handleGenerate}
                  isLoading={loadingKey === `${candidate.repo}#${candidate.number}`}
                />
              ))}
            </div>
          </section>
        )}

        {grouped.done.length > 0 && (
          <section>
            <h2 className="text-text-secondary mb-2 text-xs font-bold tracking-wider uppercase">
              Already reviewed ({grouped.done.length})
            </h2>
            <div className="flex flex-col gap-2">
              {grouped.done.map(candidate => (
                <CandidateRow
                  key={`${candidate.repo}#${candidate.number}`}
                  candidate={candidate}
                  onGenerate={handleGenerate}
                  isLoading={loadingKey === `${candidate.repo}#${candidate.number}`}
                />
              ))}
            </div>
          </section>
        )}
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
}

const CandidateRow: React.FC<CandidateRowProps> = ({ candidate, onGenerate, isLoading }) => {
  const { openUrl } = useTauri();
  const { label, tone, canGenerate } = statusLabel(candidate.status);

  return (
    <div className="border-border bg-bg-secondary/30 hover:bg-bg-secondary flex items-center gap-3 rounded-lg border px-3 py-2.5 transition-colors">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-text-disabled font-mono text-[10px]">
            {candidate.repo}#{candidate.number}
          </span>
          {candidate.isDraft && (
            <span className="text-text-disabled border-border rounded border px-1 text-[10px]">
              DRAFT
            </span>
          )}
          <span className={`text-[10px] ${tone}`}>{label}</span>
        </div>
        <p className="text-text-primary mt-0.5 truncate text-sm">{candidate.title}</p>
        <p className="text-text-secondary mt-0.5 text-[11px]">
          {candidate.author} · updated {formatDistanceToNow(new Date(candidate.updatedAt))} ago
        </p>
      </div>

      <button
        onClick={() => openUrl(candidate.url)}
        className="text-text-secondary hover:text-text-primary shrink-0 rounded p-1.5 transition-colors"
        title="Open in browser"
      >
        <ICONS.ACTION_OPEN_WINDOW size={14} />
      </button>

      {canGenerate && (
        <button
          onClick={() => onGenerate(candidate)}
          disabled={isLoading}
          className="bg-brand/10 text-brand hover:bg-brand/20 border-brand/20 shrink-0 rounded border px-2.5 py-1.5 text-xs font-medium transition-colors disabled:opacity-50"
        >
          {isLoading ? 'Loading…' : 'Generate'}
        </button>
      )}
    </div>
  );
};
