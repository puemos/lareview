import React, { useMemo } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { ICONS } from '../../constants/icons';
import { toast } from 'sonner';
import { useReviewCandidates } from '../../hooks/useReviewCandidates';
import { useTauri } from '../../hooks/useTauri';
import { useAppStore } from '../../store';
import type { AnnotatedCandidate, CandidateStatus, ViewType } from '../../types';

interface InboxViewProps {
  onNavigate: (view: ViewType) => void;
}

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

  const grouped = useMemo(() => {
    const actionable = candidates.filter(c => statusLabel(c.status).canGenerate);
    const done = candidates.filter(c => !statusLabel(c.status).canGenerate);
    return { actionable, done };
  }, [candidates]);

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
            <div className="flex flex-col gap-2 opacity-60">
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
