import React from 'react';
import { ICONS } from '../../constants/icons';

interface ConfirmationModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  message: string;
  confirmLabel?: string;
  confirmVariant?: 'danger' | 'brand' | 'accent';
  isProcessing?: boolean;
}

export const ConfirmationModal: React.FC<ConfirmationModalProps> = ({
  isOpen,
  onClose,
  onConfirm,
  title,
  message,
  confirmLabel = 'Confirm',
  confirmVariant = 'danger',
  isProcessing = false,
}) => {
  if (!isOpen) return null;

  const variantStyles = {
    danger: 'bg-status-ignored hover:bg-status-ignored/90 text-white shadow-sm',
    brand: 'bg-brand hover:bg-brand/90 text-brand-fg shadow-sm',
    accent: 'bg-accent hover:bg-accent/90 text-white shadow-sm',
  };

  return (
    <div className="animate-in fade-in fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm duration-200">
      <div 
        className="bg-bg-primary border-border/50 animate-in zoom-in-95 flex w-full max-w-md flex-col rounded-xl border shadow-2xl duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="border-border/50 bg-bg-secondary/30 flex items-center justify-between rounded-t-xl border-b px-5 py-4">
          <div className="flex items-center gap-2.5">
            <div className={`rounded-md p-1.5 ${
              confirmVariant === 'danger' ? 'bg-status-ignored/10 text-status-ignored' : 
              confirmVariant === 'brand' ? 'bg-brand/10 text-brand' : 
              'bg-accent/10 text-accent'
            }`}>
              {confirmVariant === 'danger' ? <ICONS.ICON_WARNING size={18} /> : <ICONS.ICON_INFO size={18} />}
            </div>
            <h3 className="text-text-primary text-sm font-semibold">{title}</h3>
          </div>
          <button
            onClick={onClose}
            className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded p-1 transition-all"
          >
            <ICONS.ACTION_CLOSE size={18} />
          </button>
        </div>

        <div className="p-6">
          <p className="text-text-secondary text-sm leading-relaxed mb-6">
            {message}
          </p>

          <div className="flex justify-end gap-3">
            <button
              onClick={onClose}
              className="text-text-secondary hover:text-text-primary px-4 py-2 text-xs font-medium transition-colors"
              disabled={isProcessing}
            >
              Cancel
            </button>
            <button
              onClick={onConfirm}
              disabled={isProcessing}
              className={`flex min-w-[100px] items-center justify-center gap-2 rounded-lg px-6 py-2 text-xs font-semibold transition-all active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50 ${variantStyles[confirmVariant]}`}
            >
              {isProcessing ? (
                <>
                  <div className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/30 border-t-white" />
                  Processing...
                </>
              ) : (
                confirmLabel
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
