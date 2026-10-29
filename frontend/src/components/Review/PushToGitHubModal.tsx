import React, { useState, useEffect } from 'react';
import { useTauri } from '../../hooks/useTauri';
import { Dialog } from '../Common/Dialog';
import { InlineError } from '../Common/InlineError';

interface PushToVcsModalProps {
  isOpen: boolean;
  providerName?: string | null;
  onClose: () => void;
  onConfirm: () => Promise<string | void>;
}

export const PushToVcsModal: React.FC<PushToVcsModalProps> = ({
  isOpen,
  providerName,
  onClose,
  onConfirm,
}) => {
  const { openUrl } = useTauri();
  const [isProcessing, setIsProcessing] = useState(false);
  const [resultUrl, setResultUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (isOpen) {
      setResultUrl(null);
      setError(null);
    }
  }, [isOpen]);
  const handleConfirm = async () => {
    if (isProcessing) return;
    setIsProcessing(true);
    setError(null);
    try {
      const result = await onConfirm();
      if (result) setResultUrl(result);
      else onClose();
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
      title={resultUrl ? 'Feedback posted' : 'Post feedback'}
      description={
        resultUrl
          ? 'Your feedback is now on the remote review.'
          : `This publishes the selected feedback to the linked ${providerName ?? 'remote'} review.`
      }
    >
      {error && (
        <div className="mb-5">
          <InlineError title="Could not post feedback" message={error} />
        </div>
      )}
      <div className="flex justify-end gap-3">
        <button
          data-autofocus
          onClick={onClose}
          disabled={isProcessing}
          className="settings-secondary-action"
        >
          {resultUrl ? 'Close' : 'Cancel'}
        </button>
        {resultUrl ? (
          <button className="settings-secondary-action" onClick={() => openUrl(resultUrl)}>
            Open remote review
          </button>
        ) : (
          <button
            onClick={handleConfirm}
            disabled={isProcessing}
            className="bg-brand text-brand-fg rounded-md px-4 py-2 text-xs font-medium disabled:opacity-50"
          >
            {isProcessing ? 'Posting…' : 'Post feedback'}
          </button>
        )}
      </div>
    </Dialog>
  );
};
