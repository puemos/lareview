import React, { useState } from 'react';
import type { Feedback, Comment, ReviewRule, DefaultIssueCategory } from '../../types';
import { ICONS } from '../../constants/icons';
import { MarkdownRenderer } from '../ui/MarkdownRenderer';
import { useQuery } from '@tanstack/react-query';
import { useTauri } from '../../hooks/useTauri';

import { InlineError } from '../Common/InlineError';
import { displayDate } from '../../utils/dates';
import { Select } from '../Common/Select';

interface FeedbackDetailProps {
  feedback: Feedback | null;
  rulesById: Record<string, ReviewRule>;
  categoriesById: Record<string, DefaultIssueCategory>;
  comments: Comment[];
  isCommentsLoading?: boolean;
  commentsError?: unknown;
  onRetryComments?: () => void;
  onUpdateStatus: (status: Feedback['status']) => void;
  onUpdateImpact: (impact: Feedback['impact']) => void;
  onDelete: () => void;
  onAddComment: (body: string) => Promise<void>;
  isUpdatingStatus: boolean;
  isUpdatingImpact: boolean;
  isAddingComment: boolean;
  onPushToRemote?: () => void;
  remoteProviderName?: string | null;
}

const impactOptions = [
  {
    value: 'blocking',
    label: 'Blocking',
    icon: ICONS.IMPACT_BLOCKING,
    color: 'text-impact-blocking',
  },
  {
    value: 'nice_to_have',
    label: 'Nice to have',
    icon: ICONS.IMPACT_NICE_TO_HAVE,
    color: 'text-impact-nice_to_have',
  },
  {
    value: 'nitpick',
    label: 'Nitpick',
    icon: ICONS.IMPACT_NITPICK,
    color: 'text-impact-nitpick',
  },
];

interface DiffSnippetLine {
  line_number: number;
  content: string;
  prefix: string;
  is_addition: boolean;
  is_deletion: boolean;
}

interface DiffSnippet {
  file_path: string;
  hunk_header: string;
  lines: DiffSnippetLine[];
  highlighted_line: number | null;
}

const DiffSnippetViewer: React.FC<{ snippet: DiffSnippet }> = ({ snippet }) => {
  return (
    <div className="bg-bg-tertiary border-border/50 overflow-hidden rounded border font-mono text-xs">
      <div className="bg-bg-secondary/50 border-border/50 text-text-tertiary border-b px-2 py-1 text-[10px]">
        {snippet.hunk_header}
      </div>
      <div className="max-h-48 space-y-0.5 overflow-y-auto p-2">
        {snippet.lines.map((line, idx) => {
          const isHighlighted = snippet.highlighted_line === line.line_number;
          return (
            <div
              key={idx}
              className={`flex gap-1 ${
                isHighlighted ? 'bg-brand/10 border-brand -mx-2 border-l-2 px-2' : ''
              }`}
            >
              <span className="text-text-disabled w-6 text-right select-none">
                {line.line_number || ''}
              </span>
              <span
                className={
                  line.is_addition
                    ? 'text-status-done'
                    : line.is_deletion
                      ? 'text-status-ignored'
                      : 'text-text-secondary'
                }
              >
                {line.prefix}
              </span>
              <span className="text-text-primary flex-1 break-all whitespace-pre-wrap">
                {line.content}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
};

const DiffSnippetSkeleton: React.FC = () => (
  <div className="bg-bg-secondary/50 border-border/50 rounded-md border p-3">
    <div className="bg-bg-tertiary mb-2 h-4 animate-pulse rounded" />
    <div className="bg-bg-tertiary h-20 animate-pulse rounded" />
  </div>
);

export const FeedbackDetail: React.FC<FeedbackDetailProps> = ({
  feedback,
  rulesById,
  categoriesById,
  comments,
  isCommentsLoading,
  commentsError,
  onRetryComments,
  onUpdateStatus,
  onUpdateImpact,
  onDelete,
  onAddComment,
  isUpdatingStatus,
  isUpdatingImpact,
  isAddingComment,
  onPushToRemote,
  remoteProviderName,
}) => {
  const { getFeedbackDiffSnippet } = useTauri();
  const [replyText, setReplyText] = useState('');
  const [replyError, setReplyError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const rule = feedback?.rule_id ? rulesById[feedback.rule_id] : null;
  const category = feedback?.category ? categoriesById[feedback.category] : null;
  const remoteLabel = remoteProviderName ?? 'remote';
  const RemoteIcon = remoteProviderName === 'GitLab' ? ICONS.ICON_GITLAB : ICONS.ICON_GITHUB;

  const { data: diffSnippet, isLoading: isDiffLoading } = useQuery<DiffSnippet | null>({
    queryKey: ['feedback-diff', feedback?.id],
    queryFn: () => {
      if (!feedback?.id) return Promise.resolve(null);
      return getFeedbackDiffSnippet(feedback.id, 3);
    },
    enabled: !!feedback?.id && !!feedback.anchor?.file_path,
  });

  if (!feedback) {
    return (
      <div className="bg-bg-primary text-text-disabled flex h-full flex-col items-center justify-center">
        <div className="space-y-4 text-center">
          <div className="bg-bg-secondary mx-auto flex h-16 w-16 items-center justify-center rounded-2xl shadow-sm">
            <ICONS.ICON_PLAN size={32} />
          </div>
          <div>
            <h2 className="text-text-primary mb-1 text-sm font-medium">No Feedback Selected</h2>
            <p className="text-text-tertiary text-xs">Select a feedback item from the list</p>
          </div>
        </div>
      </div>
    );
  }

  const handleAddComment = async () => {
    if (!replyText.trim() || isAddingComment || sending) return;
    setSending(true);
    setReplyError(null);
    try {
      await onAddComment(replyText.trim());
      setReplyText('');
    } catch (error) {
      setReplyError(String(error));
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <div className="review-detail-header border-border border-b">
        <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0 flex-1">
            <p className="text-text-tertiary mb-2 text-xs font-medium">Review feedback</p>
            <h2 className="text-text-primary text-lg leading-snug font-semibold break-words">
              {feedback.title || 'Untitled feedback'}
            </h2>
          </div>
          <div className="ml-2 flex items-center gap-1">
            {remoteProviderName && onPushToRemote && feedback.status !== 'ignored' && (
              <button
                onClick={onPushToRemote}
                className="hover:text-text-primary text-text-tertiary flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium transition-colors hover:bg-white/5 active:scale-[0.98]"
                title={`Push comment to ${remoteLabel}`}
              >
                <RemoteIcon size={14} />
                <span>Post Comment</span>
              </button>
            )}
            {feedback.status === 'ignored' ? (
              <>
                <button
                  onClick={() => onUpdateStatus('todo')}
                  disabled={isUpdatingStatus}
                  className="text-text-tertiary hover:text-status-done hover:bg-status-done/10 flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition-colors disabled:opacity-50"
                  title="Restore this feedback"
                >
                  <ICONS.ACTION_RESTORE size={14} />
                  <span>Restore</span>
                </button>
                <button
                  onClick={onDelete}
                  className="text-text-tertiary hover:text-status-ignored hover:bg-status-ignored/10 rounded p-1.5 transition-colors"
                  title="Delete feedback permanently"
                >
                  <ICONS.ACTION_DELETE size={14} />
                </button>
              </>
            ) : (
              <button
                onClick={() => onUpdateStatus('ignored')}
                disabled={isUpdatingStatus}
                className="text-text-tertiary hover:text-status-ignored hover:bg-status-ignored/10 flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition-colors disabled:opacity-50"
                title="Ignore this feedback (helps improve future reviews)"
              >
                <ICONS.STATUS_IGNORED size={14} />
                <span>Ignore</span>
              </button>
            )}
          </div>
        </div>

        <div className="flex w-full flex-wrap items-start justify-between gap-3">
          <div className="space-y-1">
            {feedback.anchor?.file_path ? (
              <span className="text-text-tertiary font-mono text-xs break-all">
                {feedback.anchor.file_path}
                {feedback.anchor.line_number ? `:${feedback.anchor.line_number}` : ''}
              </span>
            ) : (
              <span className="text-text-tertiary font-mono text-xs break-all">General</span>
            )}
            {feedback.rule_id && (
              <span className="text-text-tertiary text-xs">
                Rule: {rule?.text || feedback.rule_id}
              </span>
            )}
            {!feedback.rule_id && feedback.category && (
              <span className="text-text-tertiary text-xs">
                Category: {category?.name || feedback.category}
              </span>
            )}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Select
              aria-label="Feedback status"
              value={feedback.status}
              onChange={value => onUpdateStatus(value as Feedback['status'])}
              disabled={isUpdatingStatus}
              options={[
                { value: 'todo', label: 'To do' },
                { value: 'in_progress', label: 'In progress' },
                { value: 'done', label: 'Done' },
                { value: 'ignored', label: 'Ignored' },
              ]}
            />
            <Select
              aria-label="Feedback impact"
              value={feedback.impact}
              onChange={val => onUpdateImpact(val as Feedback['impact'])}
              options={impactOptions}
              disabled={isUpdatingImpact}
            />
          </div>
        </div>
      </div>

      <div className="custom-scrollbar flex-1 space-y-6 overflow-y-auto p-6">
        {feedback.anchor && feedback.anchor.file_path && (
          <div className="bg-bg-secondary/50 border-border/50 rounded-md border p-3">
            <div className="mb-2 flex items-center gap-2">
              <ICONS.TAB_CHANGES size={12} className="text-text-tertiary" />
              <span className="text-text-tertiary font-mono text-xs break-all">
                {feedback.anchor.file_path}
                {feedback.anchor.line_number ? `:${feedback.anchor.line_number}` : ''}
              </span>
              <span className="text-text-disabled text-[10px]">
                ({feedback.anchor.side === 'old' ? 'old' : 'new'})
              </span>
            </div>
            {isDiffLoading ? (
              <DiffSnippetSkeleton />
            ) : diffSnippet ? (
              <DiffSnippetViewer snippet={diffSnippet} />
            ) : (
              <div className="bg-bg-tertiary text-text-disabled flex h-20 items-center justify-center rounded text-xs">
                No code excerpt available for this location.
              </div>
            )}
          </div>
        )}

        <div className="space-y-3">
          <h3 className="text-text-secondary text-xs font-medium">Comments</h3>

          {commentsError ? (
            <InlineError
              title="Could not load comments"
              message={String(commentsError)}
              onRetry={onRetryComments}
            />
          ) : isCommentsLoading ? (
            <p role="status" className="text-text-tertiary text-xs">
              Loading comments…
            </p>
          ) : comments.length === 0 ? (
            <p className="text-text-disabled text-xs">No comments yet</p>
          ) : (
            comments.map(comment => (
              <div key={comment.id} className="flex gap-3">
                <div className="bg-brand/20 flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full">
                  <span className="text-brand text-[10px] font-medium">
                    {comment.author.charAt(0).toUpperCase()}
                  </span>
                </div>
                <div className="min-w-0 flex-1">
                  <div className="mb-1 flex items-center gap-2">
                    <span className="text-text-primary text-xs font-medium">{comment.author}</span>
                    <span className="text-text-tertiary text-xs">
                      {displayDate(comment.created_at) ?? 'Unknown date'}
                    </span>
                  </div>
                  <MarkdownRenderer className="prose prose-invert prose-sm text-text-secondary max-w-none">
                    {comment.body}
                  </MarkdownRenderer>
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      <div className="border-border border-t p-4">
        {replyError && (
          <div className="mb-3">
            <InlineError title="Reply was not sent" message={replyError} />
          </div>
        )}
        <div className="review-reply border-border bg-bg-secondary relative rounded-lg border">
          <textarea
            aria-label="Reply to feedback"
            value={replyText}
            onChange={e => setReplyText(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                handleAddComment();
              }
            }}
            placeholder="Write a reply..."
            className="text-text-primary placeholder:text-text-disabled w-full resize-none bg-transparent px-3 py-2.5 text-[13px] leading-6 focus:outline-none"
            rows={Math.max(1, Math.min(5, replyText.split('\n').length))}
            style={{ minHeight: '36px' }}
            disabled={isAddingComment || sending}
          />
          <div className="flex items-center justify-between px-2 pt-1 pb-2">
            <span className="text-text-disabled px-1 text-[11px]">⌘ / Ctrl + Enter to reply</span>
            <button
              onClick={handleAddComment}
              disabled={!replyText.trim() || isAddingComment || sending}
              className="bg-brand text-brand-fg hover:bg-brand/90 rounded px-3 py-2 text-xs font-medium transition-all disabled:opacity-50"
            >
              {isAddingComment || sending ? 'Sending…' : 'Reply'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
