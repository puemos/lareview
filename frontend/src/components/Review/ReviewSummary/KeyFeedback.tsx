import React from 'react';
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
      <span className="text-text-primary min-w-0 flex-1 truncate text-sm">{feedback.title}</span>
      <ImpactBadge impact={feedback.impact} size="sm" />
    </button>
  );
};

export const KeyFeedback: React.FC<KeyFeedbackProps> = ({ feedbacks, onSelectFeedback }) => {
  // Sort by impact: blocking first, then nice_to_have, then nitpick
  const sortedFeedbacks = [...feedbacks].sort((a, b) => {
    const order = { blocking: 0, nice_to_have: 1, nitpick: 2 };
    return order[a.impact] - order[b.impact];
  });

  if (feedbacks.length === 0) {
    return (
      <div className="bg-bg-secondary/30 border-border/50 rounded-lg border">
        <div className="border-border/50 flex items-center gap-2 border-b px-4 py-3">
          <ICONS.ICON_FEEDBACK size={16} className="text-text-secondary" />
          <h3 className="text-text-primary text-sm font-medium">Feedback</h3>
        </div>
        <div className="text-text-disabled px-4 py-6 text-center text-sm">
          No feedback items yet.
        </div>
      </div>
    );
  }

  return (
    <div className="bg-bg-secondary/30 border-border/50 flex max-h-80 flex-col overflow-hidden rounded-lg border">
      <div className="border-border/50 flex flex-shrink-0 items-center justify-between border-b px-4 py-3">
        <div className="flex items-center gap-2">
          <ICONS.ICON_FEEDBACK size={16} className="text-text-secondary" />
          <h3 className="text-text-primary text-sm font-medium">Feedback</h3>
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
