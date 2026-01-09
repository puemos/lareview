import React, { useState, useEffect } from 'react';
import { ICONS } from '../../constants/icons';
import { toast } from 'sonner';
import type { ReviewTask, Feedback } from '../../types';
import { useTauri } from '../../hooks/useTauri';

export type ExportFormat = 'markdown' | 'github';

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
  isGitHubAvailable: boolean;
}

export const SelectionModal: React.FC<SelectionModalProps> = ({
  isOpen,
  onClose,
  onConfirm,
  tasks,
  feedbacks,
  isGitHubAvailable,
}) => {
  const { openUrl } = useTauri();
  const [format, setFormat] = useState<ExportFormat>('markdown');
  const [selectedTasks, setSelectedTasks] = useState<Set<string>>(new Set());
  const [selectedFeedbacks, setSelectedFeedbacks] = useState<Set<string>>(new Set());
  const [isProcessing, setIsProcessing] = useState(false);
  const [resultUrl, setResultUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Initialize selection when modal opens
  useEffect(() => {
    if (isOpen) {
      setSelectedTasks(new Set(tasks.map(t => t.id)));
      setSelectedFeedbacks(new Set(feedbacks.map(f => f.id)));
      setResultUrl(null);
      setError(null);
      setIsProcessing(false);
      // Default to GitHub if available, otherwise Markdown
      setFormat(isGitHubAvailable ? 'github' : 'markdown');
    }
  }, [isOpen, tasks, feedbacks, isGitHubAvailable]);

  if (!isOpen) return null;

  const toggleTask = (id: string) => {
    const next = new Set(selectedTasks);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelectedTasks(next);
  };

  const toggleFeedback = (id: string) => {
    const next = new Set(selectedFeedbacks);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelectedFeedbacks(next);
  };

  const handleConfirm = async () => {
    setIsProcessing(true);
    setError(null);
    try {
      const result = await onConfirm(
        format,
        Array.from(selectedTasks),
        Array.from(selectedFeedbacks)
      );
      if (format === 'github' && result) {
        setResultUrl(result);
      } else if (format === 'markdown') {
        onClose();
        toast('Copied to Clipboard', {
          description: 'Review markdown is ready to paste.',
        });
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setIsProcessing(false);
    }
  };

  if (resultUrl) {
    return (
      <div className="animate-in fade-in fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm duration-200">
        <div className="bg-bg-primary border-border/50 animate-in zoom-in-95 flex w-full max-w-sm flex-col rounded-xl border p-8 text-center shadow-2xl duration-200">
          <div className="mx-auto mb-6 flex h-16 w-16 items-center justify-center rounded-full bg-green-500/10 text-green-500 ring-1 ring-green-500/20">
            <ICONS.ICON_CHECK size={32} weight="bold" />
          </div>
          <h3 className="text-text-primary mb-2 text-xl font-bold">Review Pushed!</h3>
          <p className="text-text-secondary mb-8 text-sm leading-relaxed">
            The review has been successfully pushed to GitHub as a PR review.
          </p>
          <div className="space-y-3">
            <button
              onClick={() => resultUrl && openUrl(resultUrl)}
              className="bg-accent hover:bg-accent/90 flex w-full items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-sm font-semibold text-white shadow-sm transition-all active:scale-[0.98]"
            >
              <ICONS.ACTION_OPEN_WINDOW size={16} weight="bold" />
              Open on GitHub
            </button>
            <button
              onClick={onClose}
              className="text-text-secondary hover:text-text-primary block w-full py-2.5 text-sm font-medium transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="animate-in fade-in fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm duration-200">
      <div className="bg-bg-primary border-border/50 animate-in zoom-in-95 flex max-h-[85vh] w-full max-w-2xl flex-col rounded-xl border shadow-2xl duration-200">
        {/* Header */}
        <div className="border-border/50 bg-bg-secondary/30 flex items-center justify-between rounded-t-xl border-b px-5 py-4">
          <div className="flex items-center gap-2.5">
            <div className="bg-accent/10 text-accent rounded-md p-1.5">
              <ICONS.ACTION_EXPORT size={18} />
            </div>
            <h3 className="text-text-primary text-sm font-semibold">Export Review</h3>
          </div>
          <button
            onClick={onClose}
            className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded p-1 transition-all"
          >
            <ICONS.ACTION_CLOSE size={18} />
          </button>
        </div>

        <div className="custom-scrollbar flex-1 space-y-8 overflow-y-auto p-6">
          {/* Format Selection */}
          <section>
            <h4 className="text-text-tertiary mb-3 px-1 text-[11px] font-bold tracking-wider uppercase">
              Output Format
            </h4>
            <div className="grid grid-cols-2 gap-3">
              <button
                onClick={() => setFormat('markdown')}
                className={`group relative flex flex-row items-center gap-3 rounded-lg border px-4 py-3 transition-all ${
                  format === 'markdown'
                    ? 'border-accent/30 bg-accent/5 text-text-primary shadow-sm'
                    : 'border-border/30 bg-bg-secondary/30 text-text-secondary hover:border-border/50 hover:bg-bg-secondary'
                }`}
              >
                {format === 'markdown' && (
                  <div className="absolute top-2 right-2 flex items-center justify-center">
                    <div className="bg-accent animate-in fade-in zoom-in-50 h-1.5 w-1.5 rounded-full duration-200" />
                  </div>
                )}
                <div
                  className={`rounded-md p-2 ${format === 'markdown' ? 'bg-accent/10' : 'bg-bg-tertiary'}`}
                >
                  <ICONS.TAB_DESCRIPTION
                    size={20}
                    className={
                      format === 'markdown'
                        ? 'text-accent'
                        : 'text-text-disabled group-hover:text-text-secondary transition-colors'
                    }
                  />
                </div>
                <div className="text-left">
                  <p className="mb-0.5 text-sm font-medium">Markdown</p>
                  <p className="text-text-tertiary text-[10px]">Copy to clipboard</p>
                </div>
              </button>

              <button
                disabled={!isGitHubAvailable}
                onClick={() => setFormat('github')}
                className={`group relative flex flex-row items-center gap-3 rounded-lg border px-4 py-3 transition-all ${
                  !isGitHubAvailable
                    ? 'cursor-not-allowed border-dashed opacity-50'
                    : 'cursor-pointer'
                } ${
                  format === 'github'
                    ? 'border-accent/30 bg-accent/5 text-text-primary shadow-sm'
                    : 'border-border/30 bg-bg-secondary/30 text-text-secondary hover:border-border/50 hover:bg-bg-secondary'
                }`}
              >
                {format === 'github' && (
                  <div className="absolute top-2 right-2 flex items-center justify-center">
                    <div className="bg-accent animate-in fade-in zoom-in-50 h-1.5 w-1.5 rounded-full duration-200" />
                  </div>
                )}
                <div
                  className={`rounded-md p-2 ${format === 'github' ? 'bg-accent/10' : 'bg-bg-tertiary'}`}
                >
                  <ICONS.ICON_GITHUB
                    size={20}
                    className={
                      format === 'github'
                        ? 'text-accent'
                        : 'text-text-disabled group-hover:text-text-secondary transition-colors'
                    }
                  />
                </div>
                <div className="text-left">
                  <p className="mb-0.5 text-sm font-medium">GitHub Review</p>
                  <p className="text-text-tertiary text-[10px]">
                    {isGitHubAvailable ? 'Post as PR review' : 'Not a GitHub PR'}
                  </p>
                </div>
              </button>
            </div>
          </section>

          {/* Task Selection */}
          <section>
            <div className="mb-3 flex items-center justify-between px-1">
              <h4 className="text-text-tertiary text-[11px] font-bold tracking-wider uppercase">
                Tasks ({selectedTasks.size}/{tasks.length})
              </h4>
              <button
                onClick={() =>
                  setSelectedTasks(
                    selectedTasks.size === tasks.length ? new Set() : new Set(tasks.map(t => t.id))
                  )
                }
                className="text-accent hover:text-accent/80 text-[10px] font-medium transition-colors"
              >
                {selectedTasks.size === tasks.length ? 'Deselect All' : 'Select All'}
              </button>
            </div>
            <div className="grid grid-cols-1 gap-2">
              {tasks.length === 0 ? (
                <div className="border-border/50 rounded-lg border border-dashed p-4 text-center">
                  <p className="text-text-disabled text-xs">No review tasks</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
                  {tasks.map(task => (
                    <label
                      key={task.id}
                      className={`flex cursor-pointer items-start gap-3 rounded-lg border p-3 transition-all select-none ${
                        selectedTasks.has(task.id)
                          ? 'bg-accent/5 border-accent/20'
                          : 'bg-bg-secondary/30 border-border/30 hover:bg-bg-secondary hover:border-border/50'
                      }`}
                    >
                      <div className="relative mt-0.5 flex items-center justify-center">
                        <input
                          type="checkbox"
                          checked={selectedTasks.has(task.id)}
                          onChange={() => toggleTask(task.id)}
                          className="peer border-border/60 checked:bg-accent checked:border-accent h-4 w-4 cursor-pointer appearance-none rounded border transition-all"
                        />
                        <div className="pointer-events-none absolute h-1.5 w-1.5 rounded-full bg-white opacity-0 transition-opacity duration-200 peer-checked:opacity-100" />
                      </div>
                      <div className="min-w-0 flex-1">
                        <p
                          className={`mb-1 line-clamp-1 text-xs font-medium ${
                            selectedTasks.has(task.id) ? 'text-text-primary' : 'text-text-secondary'
                          }`}
                        >
                          {task.title}
                        </p>
                        <div className="flex items-center gap-2">
                          <span
                            className={`rounded-full px-1.5 py-0.5 text-[9px] font-medium ${
                              task.stats.risk === 'high'
                                ? 'bg-red-500/10 text-red-500'
                                : task.stats.risk === 'medium'
                                  ? 'bg-yellow-500/10 text-yellow-500'
                                  : 'bg-blue-500/10 text-blue-500'
                            }`}
                          >
                            {task.stats.risk} risk
                          </span>
                        </div>
                      </div>
                    </label>
                  ))}
                </div>
              )}
            </div>
          </section>

          {/* Feedback Selection */}
          <section>
            <div className="mb-3 flex items-center justify-between px-1">
              <h4 className="text-text-tertiary text-[11px] font-bold tracking-wider uppercase">
                Feedback ({selectedFeedbacks.size}/{feedbacks.length})
              </h4>
              <button
                onClick={() =>
                  setSelectedFeedbacks(
                    selectedFeedbacks.size === feedbacks.length
                      ? new Set()
                      : new Set(feedbacks.map(f => f.id))
                  )
                }
                className="text-accent hover:text-accent/80 text-[10px] font-medium transition-colors"
              >
                {selectedFeedbacks.size === feedbacks.length ? 'Deselect All' : 'Select All'}
              </button>
            </div>
            <div className="grid grid-cols-1 gap-2">
              {feedbacks.length === 0 ? (
                <div className="border-border/50 rounded-lg border border-dashed p-4 text-center">
                  <p className="text-text-disabled text-xs">No feedback items</p>
                </div>
              ) : (
                <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
                  {feedbacks.map(fb => (
                    <label
                      key={fb.id}
                      className={`flex cursor-pointer items-start gap-3 rounded-lg border p-3 transition-all select-none ${
                        selectedFeedbacks.has(fb.id)
                          ? 'bg-accent/5 border-accent/20'
                          : 'bg-bg-secondary/30 border-border/30 hover:bg-bg-secondary hover:border-border/50'
                      }`}
                    >
                      <div className="relative mt-0.5 flex items-center justify-center">
                        <input
                          type="checkbox"
                          checked={selectedFeedbacks.has(fb.id)}
                          onChange={() => toggleFeedback(fb.id)}
                          className="peer border-border/60 checked:bg-accent checked:border-accent h-4 w-4 cursor-pointer appearance-none rounded border transition-all"
                        />
                        <div className="pointer-events-none absolute h-1.5 w-1.5 rounded-full bg-white opacity-0 transition-opacity duration-200 peer-checked:opacity-100" />
                      </div>
                      <div className="min-w-0 flex-1">
                        <p
                          className={`mb-1 line-clamp-1 text-xs font-medium ${
                            selectedFeedbacks.has(fb.id)
                              ? 'text-text-primary'
                              : 'text-text-secondary'
                          }`}
                        >
                          {fb.title || 'Untitled Feedback'}
                        </p>
                        <div className="flex items-center gap-2">
                          <span
                            className={`rounded-full px-1.5 py-0.5 text-[9px] font-medium ${
                              fb.impact === 'blocking'
                                ? 'bg-red-500/10 text-red-500'
                                : fb.impact === 'nitpick'
                                  ? 'bg-bg-tertiary text-text-secondary'
                                  : 'bg-blue-500/10 text-blue-500'
                            }`}
                          >
                            {fb.impact}
                          </span>
                        </div>
                      </div>
                    </label>
                  ))}
                </div>
              )}
            </div>
          </section>
        </div>

        {/* Footer */}
        <div className="border-border/50 bg-bg-secondary/30 rounded-b-xl border-t p-5 backdrop-blur-sm">
          {error && (
            <div className="mb-4 flex items-center gap-2 rounded-lg border border-red-500/20 bg-red-500/10 p-3 text-xs text-red-500">
              <ICONS.ICON_WARNING size={16} />
              {error}
            </div>
          )}
          <div className="flex items-center justify-between">
            <div className="text-text-tertiary text-[11px] font-medium">
              <span className="text-text-primary">
                {selectedTasks.size + selectedFeedbacks.size}
              </span>{' '}
              items selected
            </div>
            <div className="flex items-center gap-3">
              <button
                onClick={onClose}
                className="text-text-secondary hover:text-text-primary px-4 py-2 text-xs font-medium transition-colors"
                disabled={isProcessing}
              >
                Cancel
              </button>
              <button
                onClick={handleConfirm}
                disabled={
                  isProcessing || (selectedTasks.size === 0 && selectedFeedbacks.size === 0)
                }
                className="bg-accent hover:bg-accent/90 flex min-w-[140px] items-center justify-center gap-2 rounded-lg px-6 py-2 text-xs font-semibold text-white shadow-sm transition-all active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50"
              >
                {isProcessing ? (
                  <>
                    <div className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/30 border-t-white" />
                    {format === 'github' ? 'Pushing...' : 'Generating...'}
                  </>
                ) : (
                  <>
                    {format === 'github' ? (
                      <>
                        <ICONS.ICON_GITHUB size={14} weight="bold" />
                        Push to GitHub
                      </>
                    ) : (
                      <>
                        <ICONS.ACTION_COPY size={14} weight="bold" />
                        Copy Markdown
                      </>
                    )}
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
