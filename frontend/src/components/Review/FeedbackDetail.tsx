import React, { useState, useEffect } from 'react';
import type { Feedback, Comment } from '../../types';
import { ICONS } from '../../constants/icons';
import ReactMarkdown from 'react-markdown';
import { useQuery } from '@tanstack/react-query';
import { useTauri } from '../../hooks/useTauri';

import { Select } from '../Common/Select';

interface FeedbackDetailProps {
  feedback: Feedback | null;
  comments: Comment[];
  onUpdateStatus: (status: Feedback['status']) => void;
  onUpdateImpact: (impact: Feedback['impact']) => void;
  onDelete: () => void;
  onAddComment: (body: string) => void;
  isUpdatingStatus: boolean;
  isUpdatingImpact: boolean;
  isAddingComment: boolean;
  onPushToGitHub?: () => void;
  isGitHubReview?: boolean;
}

const statusOptions = [
  {
    value: 'todo',
    label: 'Todo',
    icon: ICONS.STATUS_TODO,
    color: 'text-status-todo',
  },
  {
    value: 'in_progress',
    label: 'In Progress',
    icon: ICONS.STATUS_IN_PROGRESS,
    color: 'text-status-in_progress',
  },
  {
    value: 'done',
    label: 'Done',
    icon: ICONS.STATUS_DONE,
    color: 'text-status-done',
  },
  {
    value: 'ignored',
    label: 'Ignored',
    icon: ICONS.STATUS_IGNORED,
    color: 'text-status-ignored',
  },
];

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

function formatTimestamp(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleString();
}

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
                    ? 'text-status-added'
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
  comments,
  onUpdateStatus,
  onUpdateImpact,
  onDelete,
  onAddComment,
  isUpdatingStatus,
  isUpdatingImpact,
  isAddingComment,
  onPushToGitHub,
  isGitHubReview,
}) => {
  const { getFeedbackDiffSnippet } = useTauri();
  const [replyText, setReplyText] = useState('');
  const [isTitleEditing, setIsTitleEditing] = useState(false);
  const [titleValue, setTitleValue] = useState('');

  const { data: diffSnippet, isLoading: isDiffLoading } = useQuery<DiffSnippet | null>({
    queryKey: ['feedback-diff', feedback?.id],
    queryFn: () => {
      if (!feedback?.id) return Promise.resolve(null);
      return getFeedbackDiffSnippet(feedback.id, 3);
    },
    enabled: !!feedback?.id && !!feedback.anchor?.file_path,
  });

  useEffect(() => {
    if (feedback) {
      setTitleValue(feedback.title);
    }
  }, [feedback]);

  if (!feedback) {
    return (
      <div className="bg-bg-primary text-text-disabled flex h-full flex-col items-center justify-center">
        <div className="space-y-4 text-center opacity-50">
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

  const handleTitleEdit = () => {
    setTitleValue(feedback.title);
    setIsTitleEditing(true);
  };

  const handleTitleSave = () => {
    if (titleValue.trim() !== feedback.title) {
      console.log('Title update would go here:', titleValue);
    }
    setIsTitleEditing(false);
  };

  const handleAddComment = () => {
    if (replyText.trim()) {
      onAddComment(replyText.trim());
      setReplyText('');
    }
  };

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <div className="border-border bg-bg-secondary/50 border-b px-4 py-3">
        <div className="mb-3 flex items-center justify-between">
          {isTitleEditing ? (
            <input
              type="text"
              value={titleValue}
              onChange={e => setTitleValue(e.target.value)}
              onBlur={handleTitleSave}
              onKeyDown={e => e.key === 'Enter' && handleTitleSave()}
              className="bg-bg-tertiary border-border text-text-primary focus:border-brand flex-1 rounded border px-2 py-1 text-sm focus:outline-none"
              autoFocus
            />
          ) : (
            <h2
              onClick={handleTitleEdit}
              className="text-text-primary hover:text-brand flex-1 cursor-pointer truncate text-sm font-medium"
            >
              {feedback.title || 'Untitled Feedback'}
            </h2>
          )}
          <div className="ml-2 flex items-center gap-1">
            {isGitHubReview && onPushToGitHub && (
              <button
                onClick={onPushToGitHub}
                className="hover:text-text-primary text-text-tertiary flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium transition-colors hover:bg-white/5 active:scale-[0.98]"
                title="Push comment to GitHub"
              >
                <ICONS.ICON_GITHUB size={14} />
                <span>Push Comment</span>
              </button>
            )}
            <button
              onClick={onDelete}
              className="text-text-tertiary hover:text-status-ignored hover:bg-status-ignored/10 rounded p-1.5 transition-colors"
              title="Delete feedback"
            >
              <ICONS.ACTION_DELETE size={14} />
            </button>
          </div>
        </div>

        <div className="flex w-full items-center justify-between">
          {feedback.anchor?.file_path ? (
            <div>
              <span className="text-text-tertiary font-mono text-[10px]">
                {feedback.anchor.file_path}:{feedback.anchor.line_number}
              </span>
            </div>
          ) : (
            <div>
              <span className="text-text-tertiary font-mono text-[10px]">General</span>
            </div>
          )}
          <div className="flex items-center gap-2">
            <Select
              value={feedback.status}
              onChange={val => onUpdateStatus(val as Feedback['status'])}
              options={statusOptions}
              disabled={isUpdatingStatus}
            />

            <Select
              value={feedback.impact}
              onChange={val => onUpdateImpact(val as Feedback['impact'])}
              options={impactOptions}
              disabled={isUpdatingImpact}
            />
          </div>
        </div>
      </div>

      <div className="custom-scrollbar flex-1 space-y-4 overflow-y-auto p-4">
        {feedback.anchor && feedback.anchor.file_path && (
          <div className="bg-bg-secondary/50 border-border/50 rounded-md border p-3">
            <div className="mb-2 flex items-center gap-2">
              <ICONS.TAB_CHANGES size={12} className="text-text-tertiary" />
              <span className="text-text-tertiary font-mono text-[10px]">
                {feedback.anchor.file_path}:{feedback.anchor.line_number}
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
                Unable to load diff snippet
              </div>
            )}
          </div>
        )}

        <div className="space-y-3">
          <h3 className="text-text-secondary text-xs font-medium">Comments</h3>

          {comments.length === 0 ? (
            <p className="text-text-disabled text-xs italic opacity-50">No comments yet</p>
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
                    <span className="text-text-tertiary text-[10px]">
                      {formatTimestamp(comment.created_at)}
                    </span>
                  </div>
                  <div className="prose prose-invert prose-sm text-text-secondary max-w-none">
                    <ReactMarkdown>{comment.body}</ReactMarkdown>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      <div className="border-border border-t p-3">
        <div className="relative rounded-lg border border-border bg-bg-secondary shadow-sm transition-all focus-within:border-brand/50 focus-within:ring-1 focus-within:ring-brand/50">
          <textarea
            value={replyText}
            onChange={e => setReplyText(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                handleAddComment();
              }
            }}
            placeholder="Write a reply..."
            className="w-full resize-none bg-transparent px-3 py-2.5 text-xs text-text-primary placeholder:text-text-disabled focus:outline-none"
            rows={Math.max(1, Math.min(5, replyText.split('\n').length))}
            style={{ minHeight: '36px' }}
            disabled={isAddingComment}
          />
          <div className="flex items-center justify-between px-2 pb-2 pt-1">
            <span className="px-1 text-[10px] text-text-disabled opacity-0 transition-opacity focus-within:opacity-100 group-focus-within:opacity-100">
              {replyText.length > 0 ? '⌘ + Enter to send' : ''}
            </span>
            <button
              onClick={handleAddComment}
              disabled={!replyText.trim() || isAddingComment}
              className="rounded bg-brand px-2.5 py-1 text-[10px] font-medium text-brand-fg transition-all hover:bg-brand/90 disabled:opacity-50"
            >
              {isAddingComment ? 'Sending...' : 'Reply'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
