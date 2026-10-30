import React, { useState, useEffect } from 'react';
import { Dialog } from '../Common/Dialog';
import { InlineError } from '../Common/InlineError';
import { ICONS } from '../../constants/icons';
import { Select } from '../Common/Select';

interface AddFeedbackModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAdd: (
    title: string,
    impact: 'blocking' | 'nice_to_have' | 'nitpick',
    content: string
  ) => Promise<void>;
  context: {
    type: 'global' | 'line';
    file?: string;
    line?: number;
  };
  isAdding: boolean;
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

export const AddFeedbackModal: React.FC<AddFeedbackModalProps> = ({
  isOpen,
  onClose,
  onAdd,
  context,
  isAdding,
}) => {
  const [title, setTitle] = useState('');
  const [impact, setImpact] = useState<'blocking' | 'nice_to_have' | 'nitpick'>('nitpick');
  const [content, setContent] = useState('');

  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  useEffect(() => {
    if (isOpen) {
      setTitle('');
      setContent('');
      setImpact('nitpick');
      setError(null);
    }
  }, [isOpen]);
  const busy = isAdding || submitting;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !content.trim() || busy) return;
    setSubmitting(true);
    setError(null);
    try {
      await onAdd(title.trim(), impact, content.trim());
    } catch (error) {
      setError(String(error));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      busy={busy}
      title="Add feedback"
      description="Describe an actionable issue. Markdown is supported."
    >
      <form onSubmit={handleSubmit} className="product-form">
        <fieldset disabled={busy} className="min-w-0 space-y-5">
          {context.type !== 'global' && (
            <div className="bg-bg-secondary/50 border-border/50 rounded border p-2">
              <div className="text-text-tertiary mb-1 text-xs">Context</div>
              <div className="text-text-primary flex items-center gap-2 font-mono text-sm">
                <ICONS.TAB_CHANGES size={14} />
                <span>
                  {context.file}:{context.line}
                </span>
              </div>
            </div>
          )}

          <div className="space-y-1">
            <label htmlFor="feedback-title" className="text-text-secondary text-xs font-medium">
              Title
            </label>
            <input
              id="feedback-title"
              required
              data-autofocus
              type="text"
              value={title}
              onChange={e => setTitle(e.target.value)}
              placeholder="Brief summary of the issue"
              className="border-border bg-bg-tertiary text-text-primary placeholder:text-text-disabled focus:border-brand w-full rounded border px-3 py-2 text-sm focus:outline-none"
            />
          </div>

          <div className="space-y-1">
            <label className="text-text-secondary text-xs font-medium">Impact</label>
            <Select
              aria-label="Feedback impact"
              value={impact}
              onChange={val => setImpact(val as typeof impact)}
              options={impactOptions}
              className="w-full"
            />
          </div>

          <div className="space-y-1">
            <label htmlFor="feedback-content" className="text-text-secondary text-xs font-medium">
              Details
            </label>
            <textarea
              id="feedback-content"
              required
              value={content}
              onChange={e => setContent(e.target.value)}
              placeholder="Describe the issue in detail (Markdown supported)"
              className="border-border bg-bg-tertiary text-text-primary placeholder:text-text-disabled focus:border-brand min-h-[120px] w-full resize-y rounded border px-3 py-2 text-sm focus:outline-none"
            />
          </div>

          {error && <InlineError title="Feedback was not added" message={error} />}
          <div className="flex justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="text-text-secondary hover:text-text-primary hover:bg-bg-secondary cursor-pointer rounded px-3 py-1.5 text-xs font-medium transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!title.trim() || !content.trim() || isAdding}
              className="text-brand-fg bg-brand hover:bg-brand/90 flex cursor-pointer items-center gap-2 rounded px-3 py-1.5 text-xs font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50"
            >
              {busy ? 'Adding…' : 'Add feedback'}
            </button>
          </div>
        </fieldset>
      </form>
    </Dialog>
  );
};
