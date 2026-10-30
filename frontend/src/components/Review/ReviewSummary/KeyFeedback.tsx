import React from 'react';
import { sortFeedback } from '../../../lib/feedback-order';
import { ICONS } from '../../../constants/icons';
import { ImpactBadge } from '../../Common/ImpactBadge';
import type { Feedback } from '../../../types';

interface KeyFeedbackProps {
  feedbacks: Feedback[];
  onSelectFeedback: (id: string) => void;
}

interface FeedbackItemProps {
  feedback: Feedback;
  onClick: () => void;
}

const FeedbackItem: React.FC<FeedbackItemProps> = ({ feedback, onClick }) => {
  return (
    <button
      onClick={onClick}
      className="hover:bg-bg-tertiary/30 border-border/30 flex w-full items-center gap-2 border-b px-3 py-2 text-left transition-colors last:border-b-0"
    >
      <ICONS.ICON_FEEDBACK size={14} className="text-text-secondary flex-shrink-0" />
      <span className="text-text-primary min-w-0 flex-1 text-[13px] leading-5">
        {feedback.title}
      </span>
      <span className="flex shrink-0 flex-col items-end gap-1">
        <ImpactBadge impact={feedback.impact} size="sm" />
        <span className="text-text-tertiary text-[11px]">
          {feedback.status === 'todo' ? 'To do' : feedback.status.replace('_', ' ')}
        </span>
      </span>
    </button>
  );
};

export const KeyFeedback: React.FC<KeyFeedbackProps> = ({ feedbacks, onSelectFeedback }) => {
  const sortedFeedbacks = sortFeedback(feedbacks);

  if (feedbacks.length === 0) {
    return (
      <div className="review-section rounded-lg border">
        <div className="review-section-heading">
          <div className="flex items-center gap-2">
            <ICONS.ICON_FEEDBACK size={14} className="text-text-tertiary" />
            <h3 className="text-text-primary text-[13px] font-medium">Feedback</h3>
          </div>
        </div>
        <div className="flex items-start gap-3 px-4 py-4">
          <ICONS.ICON_FEEDBACK size={16} className="text-text-disabled mt-0.5 shrink-0" />
          <div>
            <p className="text-text-secondary text-xs">No feedback items yet</p>
            <p className="text-text-disabled mt-1 text-[11px] leading-5">
              Feedback you add during review appears here.
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="review-section flex max-h-80 flex-col overflow-hidden rounded-lg border">
      <div className="review-section-heading">
        <div className="flex items-center gap-2">
          <ICONS.ICON_FEEDBACK size={14} className="text-text-tertiary" />
          <h3 className="text-text-primary text-[13px] font-medium">Feedback</h3>
        </div>
        <span className="text-text-disabled text-xs">
          {feedbacks.length} {feedbacks.length === 1 ? 'item' : 'items'}
        </span>
      </div>
      <div className="flex-1 overflow-y-auto">
        {sortedFeedbacks.map(feedback => (
          <FeedbackItem
            key={feedback.id}
            feedback={feedback}
            onClick={() => onSelectFeedback(feedback.id)}
          />
        ))}
      </div>
    </div>
  );
};
