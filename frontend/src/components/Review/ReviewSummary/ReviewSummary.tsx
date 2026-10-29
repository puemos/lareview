import React, { useMemo } from 'react';
import './reviewSummary.css';
import ReactMarkdown from 'react-markdown';
import { ArrowRight } from '@phosphor-icons/react';
import { ICONS } from '../../../constants/icons';
import { TaskFlow } from './TaskFlow';
import { IssueChecklist } from './IssueChecklist';
import { KeyFeedback } from './KeyFeedback';
import { FilesHeatmap } from './FilesHeatmap';
import { UncoveredFiles } from './UncoveredFiles';
import { MergeConfidenceBadge } from './MergeConfidenceBadge';
import { useIssueChecks } from '../../../hooks/useIssueChecks';
import { useTauri } from '../../../hooks/useTauri';
import { countOpenBlockers } from '../../../lib/review-summary';
import { sourceUrl } from './sourceUrl';
import { ReviewUsagePopover } from '../ReviewUsagePopover';
import { reviewHeaderControlClass } from '../reviewHeaderStyles';
import type {
  ReviewTask,
  Feedback,
  ParsedDiff,
  Review,
  ReviewSource,
  ReviewRun,
} from '../../../types';

interface ReviewSummaryProps {
  runId: string | undefined;
  run?: ReviewRun | null;
  tasks: ReviewTask[];
  feedbacks: Feedback[];
  parsedDiff: ParsedDiff | null;
  review: Review | null;
  onSelectFeedback: (id: string) => void;
  onSelectFile: (fileName: string) => void;
  onSelectTask: (taskId: string) => void;
  onStartReview: () => void;
}

interface SourceBadgeProps {
  source: ReviewSource;
}

const SourceBadge: React.FC<SourceBadgeProps> = ({ source }) => {
  const { openUrl } = useTauri();

  if (source.type === 'diff_paste') {
    return <span className="text-text-disabled text-xs">Diff paste</span>;
  }

  if (source.type === 'local_git') {
    return (
      <span
        className="text-text-tertiary text-xs break-words"
        title={`${source.base_sha} → ${source.head_sha}`}
      >
        Local Git · {source.base_ref} → {source.head_ref}
      </span>
    );
  }

  const isGitHub = source.type === 'github_pr';
  const label = isGitHub
    ? `${source.owner}/${source.repo}#${source.number}`
    : `${source.project_path}!${source.number}`;
  const url = sourceUrl(source);

  const content = (
    <>
      {isGitHub ? (
        <ICONS.ICON_GITHUB size={14} className="text-text-secondary" />
      ) : (
        <ICONS.ICON_GITLAB size={14} className="text-text-secondary" />
      )}
      <span className="text-text-secondary truncate">{label}</span>
    </>
  );

  if (!url) {
    return <span className="flex items-center gap-1.5 text-xs">{content}</span>;
  }

  return (
    <button
      type="button"
      onClick={() => openUrl(url)}
      title={`Open ${label} in browser`}
      aria-label={`Open ${label} in browser`}
      className="text-text-secondary hover:text-text-primary hover:bg-bg-tertiary group -mx-1.5 flex max-w-full items-center gap-1.5 rounded px-1.5 py-0.5 text-xs transition-colors"
    >
      {content}
      <ICONS.ACTION_OPEN_WINDOW
        size={12}
        className="text-text-disabled group-hover:text-text-secondary"
      />
    </button>
  );
};

export const ReviewSummary: React.FC<ReviewSummaryProps> = ({
  runId,
  run,
  tasks,
  feedbacks,
  parsedDiff,
  review,
  onSelectFeedback,
  onSelectFile,
  onSelectTask,
  onStartReview,
}) => {
  const {
    data: issueChecks = [],
    isLoading: isChecksLoading,
    error: checksError,
    refetch: refetchChecks,
  } = useIssueChecks(runId);

  const uncoveredFiles = useMemo(() => {
    const allDiffFiles = parsedDiff?.files?.map(f => f.new_path) ?? [];
    const coveredFiles = new Set(tasks.flatMap(t => t.files));
    return allDiffFiles.filter(f => !coveredFiles.has(f));
  }, [parsedDiff, tasks]);

  const fileCount = parsedDiff?.files?.length ?? new Set(tasks.flatMap(task => task.files)).size;

  const blockingCount = countOpenBlockers(feedbacks, issueChecks);

  return (
    <div
      className="flex h-full min-w-0 flex-col overflow-hidden"
      role="region"
      aria-label="Review overview"
    >
      {/* Header */}
      <div className="border-border/50 flex shrink-0 flex-wrap items-center justify-between gap-4 border-b px-8 py-6">
        <div className="min-w-0 flex-1">
          {review?.source && <SourceBadge source={review.source} />}
          <h2
            className="text-text-primary mt-2 text-[22px] leading-tight font-semibold tracking-[-0.025em] break-words"
            title={review?.title || parsedDiff?.title || 'Review Summary'}
          >
            {review?.title || parsedDiff?.title || 'Review Summary'}
          </h2>
          <p className="text-text-tertiary mt-2 text-xs">
            {tasks.length} {tasks.length === 1 ? 'task' : 'tasks'}
            <span className="mx-2">·</span>
            {fileCount} {fileCount === 1 ? 'file' : 'files'}
            <span className="mx-2">·</span>
            {feedbacks.length} feedback
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {run && <ReviewUsagePopover run={run} />}
          <MergeConfidenceBadge runId={runId} />
          <button
            type="button"
            onClick={onStartReview}
            className={`${reviewHeaderControlClass} bg-brand text-brand-fg hover:bg-brand/90 border-transparent px-4 text-sm font-medium`}
          >
            Open review
            <ArrowRight size={16} className="opacity-70" />
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-[1120px] space-y-6 px-8 py-8">
          {/* Blocking Issues */}
          {blockingCount > 0 && (
            <div className="bg-impact-blocking/5 border-impact-blocking/20 flex items-center gap-3 rounded-lg border px-4 py-3">
              <div className="bg-impact-blocking/10 flex h-8 w-8 items-center justify-center rounded-full">
                <span className="text-impact-blocking text-lg font-bold">{blockingCount}</span>
              </div>
              <div>
                <p className="text-impact-blocking text-sm font-medium">
                  {blockingCount} open blocking {blockingCount === 1 ? 'issue' : 'issues'}
                </p>
                <p className="text-text-tertiary text-xs">Review these items before approving</p>
              </div>
            </div>
          )}

          {/* Summary */}
          {review?.summary && (
            <div className="border-border/50 relative border-b pb-7">
              <h3 className="text-text-primary mb-3 text-[13px] font-medium">Summary</h3>
              <div className="prose prose-sm prose-invert text-text-secondary prose-p:my-2 max-w-[78ch] text-[14px] leading-6">
                <ReactMarkdown>{review.summary}</ReactMarkdown>
              </div>
            </div>
          )}

          {/* Two-column layout for Task Flow and Files Heatmap */}
          <div className="grid grid-cols-1 items-start gap-5 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)]">
            <TaskFlow tasks={tasks} onSelectTask={onSelectTask} />
            <FilesHeatmap diffFiles={parsedDiff?.files} tasks={tasks} onSelectFile={onSelectFile} />
          </div>

          {/* Two-column layout for Issue Checklist and Key Feedback */}
          <div className="grid grid-cols-1 items-start gap-5 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)]">
            <IssueChecklist
              checks={issueChecks}
              error={checksError}
              onRetry={() => void refetchChecks()}
              isLoading={isChecksLoading}
              feedbacks={feedbacks}
              onSelectFeedback={onSelectFeedback}
            />
            <KeyFeedback feedbacks={feedbacks} onSelectFeedback={onSelectFeedback} />
          </div>

          {/* Uncovered Files */}
          {uncoveredFiles.length > 0 && (
            <UncoveredFiles uncoveredFiles={uncoveredFiles} onSelectFile={onSelectFile} />
          )}
        </div>
      </div>
    </div>
  );
};
