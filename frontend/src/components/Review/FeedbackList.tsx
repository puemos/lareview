import React from 'react';
import type { Feedback, ReviewRule, DefaultIssueCategory } from '../../types';
import { ICONS } from '../../constants/icons';
import { RulePopover } from './RulePopover';
import { CategoryBadge } from './CategoryBadge';

interface FeedbackListProps {
  feedbacks: Feedback[];
  rulesById: Record<string, ReviewRule>;
  categoriesById: Record<string, DefaultIssueCategory>;
  selectedFeedbackId: string | null;
  onSelectFeedback: (feedbackId: string) => void;
  isLoading?: boolean;
  /** Minimum confidence threshold for displaying feedback (null = show all) */
  confidenceThreshold?: number | null;
}

const FeedbackSkeleton: React.FC = () => (
  <div className="border-border/50 w-full border-b px-4 py-3">
    <div className="flex items-start gap-2.5">
      <div className="bg-bg-tertiary mt-0.5 h-3.5 w-3.5 animate-pulse rounded-full" />
      <div className="min-w-0 flex-1 space-y-1.5">
        <div className="bg-bg-tertiary h-3 w-3/4 animate-pulse rounded" />
        <div className="bg-bg-tertiary h-2 w-1/2 animate-pulse rounded" />
        <div className="flex items-center gap-2">
          <div className="bg-bg-tertiary h-2 w-12 animate-pulse rounded" />
          <span className="text-text-disabled text-[10px]">·</span>
          <div className="bg-bg-tertiary h-2 w-16 animate-pulse rounded" />
        </div>
      </div>
    </div>
  </div>
);

const IMPACT_CONFIG = {
  blocking: { icon: ICONS.IMPACT_BLOCKING, color: 'text-impact-blocking' },
  nice_to_have: {
    icon: ICONS.IMPACT_NICE_TO_HAVE,
    color: 'text-impact-nice_to_have',
  },
  nitpick: { icon: ICONS.IMPACT_NITPICK, color: 'text-impact-nitpick' },
};


export const FeedbackList: React.FC<FeedbackListProps> = ({
  feedbacks,
  rulesById,
  categoriesById,
  selectedFeedbackId,
  onSelectFeedback,
  isLoading = false,
  confidenceThreshold = null,
}) => {
  if (isLoading) {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto">
        {[1, 2, 3, 4, 5].map(i => (
          <FeedbackSkeleton key={i} />
        ))}
      </div>
    );
  }

  // Filter feedback based on confidence threshold
  const filteredFeedbacks =
    confidenceThreshold != null
      ? feedbacks.filter(f => f.confidence >= confidenceThreshold)
      : feedbacks;
  const hiddenCount = feedbacks.length - filteredFeedbacks.length;

  if (filteredFeedbacks.length === 0) {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto">
        <div className="p-4 text-center">
          <ICONS.ICON_PLAN size={24} className="text-text-disabled mx-auto mb-2 opacity-50" />
          <p className="text-text-disabled text-xs opacity-50">No feedback yet</p>
          <p className="text-text-tertiary mt-1 text-[10px]">
            {hiddenCount > 0
              ? `${hiddenCount} low-confidence item${hiddenCount > 1 ? 's' : ''} hidden`
              : 'Add inline feedback from the diff view'}
          </p>
        </div>
      </div>
    );
  }

  const sortedFeedbacks = [...filteredFeedbacks].sort((a, b) => {
    const statusRank = { todo: 0, in_progress: 1, done: 2, ignored: 3 };
    const rankA = statusRank[a.status] ?? 0;
    const rankB = statusRank[b.status] ?? 0;
    if (rankA !== rankB) return rankA - rankB;
    return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
  });

  return (
    <div className="custom-scrollbar flex-1 overflow-y-auto">
      {sortedFeedbacks.map(feedback => {
        const isActive = selectedFeedbackId === feedback.id;
        const impact =
          IMPACT_CONFIG[feedback.impact as keyof typeof IMPACT_CONFIG] || IMPACT_CONFIG.nitpick;
        const rule =
          feedback.rule_id && rulesById[feedback.rule_id] ? rulesById[feedback.rule_id] : null;
        const category =
          feedback.category && categoriesById[feedback.category]
            ? categoriesById[feedback.category]
            : null;
        const hasRuleBadge = feedback.rule_id;
        const hasCategoryBadge = !hasRuleBadge && feedback.category;

        return (
          <button
            key={feedback.id}
            onClick={() => onSelectFeedback(feedback.id)}
            className={`group border-border/50 hover:bg-bg-secondary/80 relative w-full border-b px-4 py-3 text-left transition-all ${
              isActive ? 'bg-bg-secondary shadow-inner' : ''
            }`}
          >
            {isActive && <div className="bg-brand absolute top-0 bottom-0 left-0 w-[2px]" />}
            <div className="flex w-full min-w-0 items-center gap-2.5">
              <div className="flex-shrink-0">
                <impact.icon size={14} className={impact.color} />
              </div>
              <h3
                className={`flex-1 truncate text-xs leading-relaxed font-medium ${
                  isActive
                    ? 'text-text-primary'
                    : 'text-text-secondary group-hover:text-text-primary'
                } ${feedback.status === 'done' || feedback.status === 'ignored' ? 'text-text-disabled line-through opacity-50' : ''}`}
              >
                {feedback.title || 'Untitled Feedback'}
              </h3>
              {hasRuleBadge && <RulePopover rule={rule} ruleId={feedback.rule_id || ''} />}
              {hasCategoryBadge && (
                <CategoryBadge category={category} categoryId={feedback.category || ''} />
              )}
            </div>
          </button>
        );
      })}
      {hiddenCount > 0 && (
        <div className="text-text-tertiary px-4 py-2 text-center text-[10px]">
          {hiddenCount} low-confidence item{hiddenCount > 1 ? 's' : ''} hidden
        </div>
      )}
    </div>
  );
};
