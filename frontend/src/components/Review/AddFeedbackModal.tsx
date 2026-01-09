import React, { useState } from 'react';
import { ICONS } from '../../constants/icons';
import { Select } from '../Common/Select';

interface AddFeedbackModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAdd: (
    title: string,
    impact: 'blocking' | 'nice_to_have' | 'nitpick',
    content: string
  ) => void;
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

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !content.trim()) return;
    onAdd(title, impact, content);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="bg-bg-primary border-border w-full max-w-lg rounded-xl border shadow-xl">
        <div className="border-border flex items-center justify-between border-b px-4 py-3">
          <h2 className="text-text-primary text-sm font-medium">Add Feedback</h2>
          <button
            onClick={onClose}
            className="text-text-tertiary hover:text-text-primary transition-colors cursor-pointer"
          >
            <ICONS.ACTION_CLOSE size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-4 space-y-4">
          {context.type !== 'global' && (
            <div className="bg-bg-secondary/50 border-border/50 rounded border p-2">
              <div className="text-xs text-text-tertiary mb-1">Context</div>
              <div className="flex items-center gap-2 text-sm text-text-primary font-mono">
                <ICONS.TAB_CHANGES size={14} />
                <span>
                  {context.file}:{context.line}
                </span>
              </div>
            </div>
          )}

          <div className="space-y-1">
            <label className="text-xs text-text-secondary font-medium">Title</label>
            <input
              type="text"
              value={title}
              onChange={e => setTitle(e.target.value)}
              placeholder="Brief summary of the issue"
              className="w-full rounded border border-border bg-bg-tertiary px-3 py-2 text-sm text-text-primary placeholder:text-text-disabled focus:border-brand focus:outline-none"
              autoFocus
            />
          </div>

          <div className="space-y-1">
            <label className="text-xs text-text-secondary font-medium">Impact</label>
            <Select
              value={impact}
              onChange={val => setImpact(val as typeof impact)}
              options={impactOptions}
              className="w-full"
            />
          </div>

          <div className="space-y-1">
            <label className="text-xs text-text-secondary font-medium">Details</label>
            <textarea
              value={content}
              onChange={e => setContent(e.target.value)}
              placeholder="Describe the issue in detail (Markdown supported)"
              className="w-full rounded border border-border bg-bg-tertiary px-3 py-2 text-sm text-text-primary placeholder:text-text-disabled focus:border-brand focus:outline-none min-h-[120px] resize-y"
            />
          </div>

          <div className="flex justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 text-xs font-medium text-text-secondary hover:text-text-primary hover:bg-bg-secondary rounded transition-colors cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!title.trim() || !content.trim() || isAdding}
              className="px-3 py-1.5 text-xs font-medium text-brand-fg bg-brand hover:bg-brand/90 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2 cursor-pointer"
            >
              {isAdding ? 'Adding...' : 'Add Feedback'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
