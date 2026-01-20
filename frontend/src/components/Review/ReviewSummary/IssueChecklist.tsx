import React, { useState } from 'react';
import { ICONS } from '../../../constants/icons';
import { ImpactBadge } from '../../Common/ImpactBadge';
import type { IssueCheckWithFindings, CheckStatus, Feedback } from '../../../types';

interface IssueChecklistProps {
  checks: IssueCheckWithFindings[];
  isLoading: boolean;
  feedbacks: Feedback[];
  onSelectFeedback: (id: string) => void;
}

const statusConfig: Record<CheckStatus, { icon: typeof ICONS.CHECK; color: string; label: string }> =
  {
    found: { icon: ICONS.STATUS_ISSUES, color: 'text-impact-blocking', label: 'Issues Found' },
    not_found: { icon: ICONS.CHECK, color: 'text-status-done', label: 'No Issues' },
    not_applicable: { icon: ICONS.MINUS, color: 'text-text-disabled', label: 'N/A' },
    skipped: { icon: ICONS.STATUS_SKIPPED, color: 'text-text-disabled', label: 'Skipped' },
  };


interface CheckItemProps {
  check: IssueCheckWithFindings;
  feedbacks: Feedback[];
  onSelectFeedback: (id: string) => void;
}

const CheckItem: React.FC<CheckItemProps> = ({ check, feedbacks, onSelectFeedback }) => {
  const [expanded, setExpanded] = useState(false);
  const config = statusConfig[check.status];
  const StatusIcon = config.icon;
  const hasFindings = check.findings.length > 0;

  return (
    <div className="border-border/30 border-b last:border-b-0">
      <button
        onClick={() => hasFindings && setExpanded(!expanded)}
        className={`hover:bg-bg-tertiary/30 flex w-full items-center gap-2 px-3 py-2 text-left transition-colors ${
          hasFindings ? 'cursor-pointer' : 'cursor-default'
        }`}
        disabled={!hasFindings}
      >
        <StatusIcon size={14} className={config.color} weight="bold" />
        <span className="text-text-primary flex-1 text-sm">{check.display_name}</span>
        {check.status === 'found' && check.findings.length > 0 && (
          <span className="bg-impact-blocking/10 text-impact-blocking rounded px-1.5 py-0.5 text-[10px] font-medium">
            {check.findings.length} {check.findings.length === 1 ? 'issue' : 'issues'}
          </span>
        )}
        {hasFindings && (
          <ICONS.CHEVRON_DOWN
            size={12}
            className={`text-text-disabled transition-transform ${expanded ? 'rotate-180' : ''}`}
          />
        )}
      </button>

      {expanded && hasFindings && (
        <div className="bg-bg-tertiary/20 border-border/20 border-t">
          {check.findings.map(finding => {
            const linkedFeedback = feedbacks.find(f => f.finding_id === finding.id);

            return (
              <button
                key={finding.id}
                onClick={() => linkedFeedback && onSelectFeedback(linkedFeedback.id)}
                className={`border-border/20 flex w-full items-center gap-2 border-b px-3 py-2 text-left last:border-b-0 ${
                  linkedFeedback
                    ? 'hover:bg-bg-tertiary/30 cursor-pointer transition-colors'
                    : 'cursor-default'
                }`}
                disabled={!linkedFeedback}
              >
                <span className="text-text-primary min-w-0 flex-1 truncate text-sm">
                  {finding.title}
                </span>
                <ImpactBadge impact={finding.impact} size="sm" />
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
};

export const IssueChecklist: React.FC<IssueChecklistProps> = ({
  checks,
  isLoading,
  feedbacks,
  onSelectFeedback,
}) => {
  if (isLoading) {
    return (
      <div className="bg-bg-secondary/30 border-border/50 rounded-lg border">
        <div className="border-border/50 flex items-center gap-2 border-b px-4 py-3">
          <ICONS.STATUS_ISSUES size={16} className="text-text-secondary" />
          <h3 className="text-text-primary text-sm font-medium">Issue Checklist</h3>
        </div>
        <div className="animate-pulse space-y-2 p-4">
          {[1, 2, 3].map(i => (
            <div key={i} className="bg-bg-tertiary/50 h-8 rounded" />
          ))}
        </div>
      </div>
    );
  }

  if (checks.length === 0) {
    return (
      <div className="bg-bg-secondary/30 border-border/50 rounded-lg border">
        <div className="border-border/50 flex items-center gap-2 border-b px-4 py-3">
          <ICONS.STATUS_ISSUES size={16} className="text-text-secondary" />
          <h3 className="text-text-primary text-sm font-medium">Issue Checklist</h3>
        </div>
        <div className="text-text-disabled px-4 py-6 text-center text-sm">
          No issue checks for this review.
          <br />
          <span className="text-xs">
            Add checklist rules in Settings to enable systematic issue verification.
          </span>
        </div>
      </div>
    );
  }

  const foundCount = checks.filter(c => c.status === 'found').length;
  const totalFindings = checks.reduce((sum, c) => sum + c.findings.length, 0);

  // Sort by status: found first, then not_found, then skipped, then not_applicable
  const statusOrder: Record<CheckStatus, number> = {
    found: 0,
    not_found: 1,
    skipped: 2,
    not_applicable: 3,
  };
  const sortedChecks = [...checks].sort((a, b) => statusOrder[a.status] - statusOrder[b.status]);

  return (
    <div className="bg-bg-secondary/30 border-border/50 flex max-h-80 flex-col overflow-hidden rounded-lg border">
      <div className="border-border/50 flex flex-shrink-0 items-center justify-between border-b px-4 py-3">
        <div className="flex items-center gap-2">
          <ICONS.STATUS_ISSUES size={16} className="text-text-secondary" />
          <h3 className="text-text-primary text-sm font-medium">Issue Checklist</h3>
        </div>
        {foundCount > 0 && (
          <span className="bg-impact-blocking/10 text-impact-blocking rounded-full px-2 py-0.5 text-[10px] font-medium">
            {totalFindings} {totalFindings === 1 ? 'issue' : 'issues'} in {foundCount}{' '}
            {foundCount === 1 ? 'category' : 'categories'}
          </span>
        )}
      </div>
      <div className="divide-border/30 flex-1 divide-y overflow-y-auto">
        {sortedChecks.map(check => (
          <CheckItem
            key={check.id}
            check={check}
            feedbacks={feedbacks}
            onSelectFeedback={onSelectFeedback}
          />
        ))}
      </div>
    </div>
  );
};
