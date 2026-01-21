import React, { useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import { ICONS } from '../../../constants/icons';
import { TaskFlow } from './TaskFlow';
import { IssueChecklist } from './IssueChecklist';
import { KeyFeedback } from './KeyFeedback';
import { FilesHeatmap } from './FilesHeatmap';
import { UncoveredFiles } from './UncoveredFiles';
import { useIssueChecks } from '../../../hooks/useIssueChecks';
import type { ReviewTask, Feedback, ParsedDiff, Review, ReviewSource } from '../../../types';

interface ReviewSummaryProps {
  runId: string | undefined;
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
  if (source.type === 'github_pr') {
    return (
      <span className="flex items-center gap-1.5 text-xs">
        <ICONS.ICON_GITHUB size={14} className="text-text-secondary" />
        <span className="text-text-secondary">
          {source.owner}/{source.repo}#{source.number}
        </span>
      </span>
    );
  }
  if (source.type === 'gitlab_mr') {
    return (
      <span className="flex items-center gap-1.5 text-xs">
        <ICONS.ICON_GITLAB size={14} className="text-text-secondary" />
        <span className="text-text-secondary">
          {source.project_path}!{source.number}
        </span>
      </span>
    );
  }
  return <span className="text-text-disabled text-xs">Diff paste</span>;
};

export const ReviewSummary: React.FC<ReviewSummaryProps> = ({
  runId,
  tasks,
  feedbacks,
  parsedDiff,
  review,
  onSelectFeedback,
  onSelectFile,
  onSelectTask,
  onStartReview,
}) => {
  const [isExpanded, setIsExpanded] = React.useState(false);
  const { data: issueChecks = [], isLoading: isChecksLoading } = useIssueChecks(runId);

  const uncoveredFiles = useMemo(() => {
    const allDiffFiles = parsedDiff?.files?.map(f => f.new_path) ?? [];
    const coveredFiles = new Set(tasks.flatMap(t => t.files));
    return allDiffFiles.filter(f => !coveredFiles.has(f));
  }, [parsedDiff, tasks]);

  const blockingCount =
    feedbacks.filter(f => f.impact === 'blocking').length +
    issueChecks.reduce(
      (sum, c) => sum + c.findings.filter(f => f.impact === 'blocking').length,
      0
    );

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header */}
      <div className="border-border bg-bg-secondary/50 flex items-center justify-between border-b px-6 py-4">
        <div>
          {review?.source && <SourceBadge source={review.source} />}
          <h2 className="text-text-primary text-lg font-semibold">
            {review?.title || parsedDiff?.title || 'Review Summary'}
          </h2>
        </div>
        <button
          onClick={onStartReview}
          className="bg-brand text-brand-fg hover:bg-brand/90 flex items-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors"
        >
          Start Review
          <span className="opacity-70">→</span>
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 space-y-6 overflow-auto p-6">
        {/* Blocking Issues Alert */}
        {blockingCount > 0 && (
          <div className="bg-status-error/5 border-status-error/20 flex items-center gap-3 rounded-lg border px-4 py-3">
            <div className="bg-status-error/10 flex h-8 w-8 items-center justify-center rounded-full">
              <span className="text-status-error text-lg font-bold">{blockingCount}</span>
            </div>
            <div>
              <p className="text-status-error text-sm font-medium">
                {blockingCount} Blocking {blockingCount === 1 ? 'Issue' : 'Issues'}
              </p>
              <p className="text-status-error/70 text-xs">
                Review these items before approving
              </p>
            </div>
          </div>
        )}

        {/* Summary */}
        {review?.summary && (
          <div className="bg-bg-tertiary/30 border-border/50 relative rounded-lg border p-4">
            <h3 className="text-text-secondary mb-2 text-xs font-medium tracking-wide uppercase">
              Summary
            </h3>
            <div
              className={`prose prose-sm prose-invert max-w-none transition-all duration-300 ease-in-out ${
                !isExpanded
                  ? 'max-h-24 overflow-hidden [mask-image:linear-gradient(to_bottom,black_50%,transparent_100%)]'
                  : 'max-h-[2000px]'
              }`}
            >
              <ReactMarkdown>{review.summary}</ReactMarkdown>
            </div>
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="text-brand hover:text-brand/80 mt-2 flex items-center gap-1 text-xs font-medium transition-colors"
            >
              {isExpanded ? (
                <>
                  <ICONS.CHEVRON_UP size={12} />
                  Show less
                </>
              ) : (
                <>
                  <ICONS.CHEVRON_DOWN size={12} />
                  Show more
                </>
              )}
            </button>
          </div>
        )}

        {/* Two-column layout for Task Flow and Files Heatmap */}
        <div className="grid grid-cols-2 items-start gap-6">
          <TaskFlow tasks={tasks} onSelectTask={onSelectTask} />
          <FilesHeatmap tasks={tasks} onSelectFile={onSelectFile} />
        </div>

        {/* Two-column layout for Issue Checklist and Key Feedback */}
        <div className="grid grid-cols-2 items-start gap-6">
          <IssueChecklist
            checks={issueChecks}
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
  );
};
