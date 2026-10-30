import React, { useState, useEffect, useRef } from 'react';
import { toast } from 'sonner';
import type { ReviewTask, Feedback } from '../../types';
import { useTauri } from '../../hooks/useTauri';
import { Dialog } from '../Common/Dialog';
import { InlineError } from '../Common/InlineError';

export type ExportFormat = 'markdown' | 'remote';
interface SelectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (
    format: ExportFormat,
    selectedTasks: string[],
    selectedFeedbacks: string[]
  ) => Promise<string | void>;
  tasks: ReviewTask[];
  feedbacks: Feedback[];
  remoteProviderName?: string | null;
}

export const SelectionModal: React.FC<SelectionModalProps> = ({
  isOpen,
  onClose,
  onConfirm,
  tasks,
  feedbacks,
  remoteProviderName,
}) => {
  const { openUrl } = useTauri();
  const [format, setFormat] = useState<ExportFormat>('markdown');
  const [selectedTasks, setSelectedTasks] = useState<Set<string>>(new Set());
  const [selectedFeedbacks, setSelectedFeedbacks] = useState<Set<string>>(new Set());
  const [isProcessing, setIsProcessing] = useState(false);
  const [resultUrl, setResultUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const wasOpen = useRef(false);
  useEffect(() => {
    if (isOpen && !wasOpen.current) {
      setSelectedTasks(new Set(tasks.map(task => task.id)));
      setSelectedFeedbacks(new Set(feedbacks.map(feedback => feedback.id)));
      setResultUrl(null);
      setError(null);
      setFormat('markdown');
    }
    wasOpen.current = isOpen;
  }, [isOpen, tasks, feedbacks]);
  const selectedCount = selectedTasks.size + selectedFeedbacks.size;
  const handleConfirm = async () => {
    if (isProcessing || selectedCount === 0) return;
    setIsProcessing(true);
    setError(null);
    try {
      const result = await onConfirm(
        format,
        tasks.filter(task => selectedTasks.has(task.id)).map(task => task.id),
        feedbacks
          .filter(feedback => selectedFeedbacks.has(feedback.id))
          .map(feedback => feedback.id)
      );
      if (format === 'remote') {
        if (!result)
          throw new Error(
            'The provider did not return a review URL. Check the remote review before trying again.'
          );
        setResultUrl(result);
      } else {
        onClose();
        toast('Copied to clipboard', { description: 'Review Markdown is ready to paste.' });
      }
    } catch (error) {
      setError(String(error));
    } finally {
      setIsProcessing(false);
    }
  };
  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      busy={isProcessing}
      title={resultUrl ? 'Review posted' : 'Export review'}
      description={
        resultUrl
          ? 'Your selected items were posted to the remote review.'
          : 'Choose the tasks and feedback to include.'
      }
    >
      {resultUrl ? (
        <div className="flex justify-end gap-3">
          <button className="settings-secondary-action" onClick={onClose}>
            Close
          </button>
          <button className="settings-secondary-action" onClick={() => openUrl(resultUrl)}>
            Open remote review
          </button>
        </div>
      ) : (
        <fieldset disabled={isProcessing} className="min-w-0 space-y-6">
          <fieldset className="space-y-3">
            <legend className="text-text-secondary mb-3 text-sm font-medium">Destination</legend>
            <label className="text-text-secondary flex items-center gap-3 text-[13px]">
              <input
                type="radio"
                name="export-format"
                checked={format === 'markdown'}
                onChange={() => setFormat('markdown')}
              />
              Copy Markdown to clipboard
            </label>
            {remoteProviderName && (
              <label className="text-text-secondary flex items-center gap-3 text-[13px]">
                <input
                  type="radio"
                  name="export-format"
                  checked={format === 'remote'}
                  onChange={() => setFormat('remote')}
                />
                Post to {remoteProviderName}
              </label>
            )}
          </fieldset>
          <ExportItems
            title="Tasks"
            items={tasks}
            selected={selectedTasks}
            onChange={setSelectedTasks}
          />
          <ExportItems
            title="Feedback"
            items={feedbacks}
            selected={selectedFeedbacks}
            onChange={setSelectedFeedbacks}
          />
          {format === 'remote' && (
            <p className="text-text-secondary bg-bg-secondary rounded-md p-3 text-xs leading-6">
              This will publish {selectedCount} selected {selectedCount === 1 ? 'item' : 'items'} to
              the linked {remoteProviderName} review.
            </p>
          )}
          {error && <InlineError title="Could not export review" message={error} />}
          <div className="border-border flex flex-wrap items-center justify-between gap-3 border-t pt-5">
            <span className="text-text-tertiary text-xs">{selectedCount} selected</span>
            <div className="flex gap-3">
              <button onClick={onClose} className="settings-secondary-action">
                Cancel
              </button>
              <button
                onClick={handleConfirm}
                disabled={isProcessing || selectedCount === 0}
                className="bg-brand text-brand-fg disabled:bg-bg-tertiary disabled:text-text-disabled rounded-md px-4 py-2 text-xs font-medium"
              >
                {isProcessing
                  ? 'Exporting…'
                  : format === 'remote'
                    ? 'Post to remote'
                    : 'Copy Markdown'}
              </button>
            </div>
          </div>
        </fieldset>
      )}
    </Dialog>
  );
};

function ExportItems({
  title,
  items,
  selected,
  onChange,
}: {
  title: string;
  items: Array<{ id: string; title: string }>;
  selected: Set<string>;
  onChange: (selected: Set<string>) => void;
}) {
  return (
    <fieldset className="min-w-0">
      <legend className="text-text-primary mb-3 w-full text-sm font-medium">
        {title}{' '}
        <span className="text-text-tertiary text-xs">
          {selected.size} of {items.length}
        </span>
      </legend>
      {items.length > 0 ? (
        <>
          <button
            type="button"
            className="text-text-secondary mb-2 rounded py-1 text-xs underline underline-offset-4"
            onClick={() =>
              onChange(
                selected.size === items.length ? new Set() : new Set(items.map(item => item.id))
              )
            }
          >
            {selected.size === items.length
              ? `Deselect all ${title.toLowerCase()}`
              : `Select all ${title.toLowerCase()}`}
          </button>
          <div className="divide-border divide-y">
            {items.map(item => (
              <label
                key={item.id}
                className="text-text-secondary flex cursor-pointer items-start gap-3 py-3 text-[13px] leading-5"
              >
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4 shrink-0"
                  checked={selected.has(item.id)}
                  onChange={() => {
                    const next = new Set(selected);
                    if (next.has(item.id)) next.delete(item.id);
                    else next.add(item.id);
                    onChange(next);
                  }}
                />
                <span className="min-w-0 break-words">{item.title || 'Untitled feedback'}</span>
              </label>
            ))}
          </div>
        </>
      ) : (
        <p className="text-text-tertiary text-xs">No {title.toLowerCase()} to include.</p>
      )}
    </fieldset>
  );
}
