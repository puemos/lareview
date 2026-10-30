import React from 'react';
import { Dialog } from './Dialog';

interface ConfirmationModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  message: string;
  confirmLabel?: string;
  confirmVariant?: 'danger' | 'brand' | 'accent';
  isProcessing?: boolean;
  children?: React.ReactNode;
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
  children,
}) => (
  <Dialog isOpen={isOpen} onClose={onClose} title={title} busy={isProcessing}>
    <p className="text-text-secondary text-sm leading-6">{message}</p>
    {children && <div className="mt-5">{children}</div>}
    <div className="mt-6 flex justify-end gap-3">
      <button
        data-autofocus
        onClick={onClose}
        disabled={isProcessing}
        className="settings-secondary-action"
      >
        Cancel
      </button>
      <button
        onClick={onConfirm}
        disabled={isProcessing}
        className={`rounded-md px-4 py-2 text-xs font-medium disabled:opacity-50 ${confirmVariant === 'danger' ? 'bg-status-ignored text-white' : 'bg-brand text-brand-fg'}`}
      >
        {isProcessing ? 'Processing…' : confirmLabel}
      </button>
    </div>
  </Dialog>
);
