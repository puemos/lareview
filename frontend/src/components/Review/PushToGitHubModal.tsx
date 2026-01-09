import React, { useState, useEffect } from 'react';
import { ICONS } from '../../constants/icons';
import { useTauri } from '../../hooks/useTauri';

interface PushToGitHubModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => Promise<string | void>;
}

export const PushToGitHubModal: React.FC<PushToGitHubModalProps> = ({
  isOpen,
  onClose,
  onConfirm,
}) => {
  const { openUrl } = useTauri();
  const [isProcessing, setIsProcessing] = useState(false);
  const [resultUrl, setResultUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Reset state when modal opens
  useEffect(() => {
    if (isOpen) {
      setResultUrl(null);
      setError(null);
      setIsProcessing(false);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleConfirm = async () => {
    setIsProcessing(true);
    setError(null);
    try {
      const result = await onConfirm();
      if (result) {
        setResultUrl(result);
      } else {
        onClose();
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
          <h3 className="text-text-primary mb-2 text-xl font-bold">Feedback Pushed!</h3>
          <p className="text-text-secondary mb-8 text-sm leading-relaxed">
            The feedback has been successfully pushed as a comment to GitHub.
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
      <div className="bg-bg-primary border-border/50 animate-in zoom-in-95 flex w-full max-w-md flex-col rounded-xl border shadow-2xl duration-200">
        <div className="border-border/50 bg-bg-secondary/30 flex items-center justify-between rounded-t-xl border-b px-5 py-4">
          <div className="flex items-center gap-2.5">
            <div className="bg-accent/10 text-accent rounded-md p-1.5">
              <ICONS.ICON_GITHUB size={18} />
            </div>
            <h3 className="text-text-primary text-sm font-semibold">Push to GitHub</h3>
          </div>
          <button
            onClick={onClose}
            className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded p-1 transition-all"
          >
            <ICONS.ACTION_CLOSE size={18} />
          </button>
        </div>

        <div className="p-6">
          <div className="mb-6 flex items-start gap-4 rounded-lg border border-blue-500/10 bg-blue-500/5 p-4">
            <div className="text-accent mt-0.5">
              <ICONS.ICON_INFO size={20} />
            </div>
            <div>
              <h4 className="text-text-primary mb-1 text-sm font-medium">Confirmation</h4>
              <p className="text-text-secondary text-xs leading-relaxed">
                Are you sure you want to push this feedback as a comment to the linked GitHub PR?
              </p>
            </div>
          </div>

          {error && (
            <div className="mb-6 flex items-center gap-2 rounded-lg border border-red-500/20 bg-red-500/10 p-3 text-xs text-red-500">
              <ICONS.ICON_WARNING size={16} />
              {error}
            </div>
          )}

          <div className="flex justify-end gap-3">
            <button
              onClick={onClose}
              className="text-text-secondary hover:text-text-primary px-4 py-2 text-xs font-medium transition-colors"
              disabled={isProcessing}
            >
              Cancel
            </button>
            <button
              onClick={handleConfirm}
              disabled={isProcessing}
              className="bg-accent hover:bg-accent/90 flex min-w-[140px] items-center justify-center gap-2 rounded-lg px-6 py-2 text-xs font-semibold text-white shadow-sm transition-all active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50"
            >
              {isProcessing ? (
                <>
                  <div className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/30 border-t-white" />
                  Pushing...
                </>
              ) : (
                <>
                  <ICONS.ICON_GITHUB size={14} weight="bold" />
                  Push Comment
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
