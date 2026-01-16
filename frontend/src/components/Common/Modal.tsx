import React, { useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { ICONS } from '../../constants/icons';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  children: React.ReactNode;
  title?: string;
  className?: string;
  hideCloseButton?: boolean;
}

export const Modal: React.FC<ModalProps> = ({
  isOpen,
  onClose,
  children,
  title,
  className = '',
  hideCloseButton = false,
}) => {
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    if (isOpen) {
      window.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';
    }

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return createPortal(
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-bg-primary/95 backdrop-blur-sm animate-in fade-in duration-200"
      ref={overlayRef}
      onClick={e => {
        if (e.target === overlayRef.current) onClose();
      }}
    >
      <div className={`relative flex h-full w-full flex-col ${className}`}>
        {(title || !hideCloseButton) && (
          <div className="absolute top-4 right-4 z-50 flex items-center gap-4">
            {title && <h2 className="text-lg font-semibold text-text-primary">{title}</h2>}
            {!hideCloseButton && (
              <button
                onClick={onClose}
                className="rounded-lg bg-bg-surface p-2 text-text-tertiary shadow-lg transition-colors hover:bg-bg-secondary hover:text-text-primary border border-border/50"
              >
                <ICONS.ACTION_CLOSE size={20} />
              </button>
            )}
          </div>
        )}
        {children}
      </div>
    </div>,
    document.body
  );
};
