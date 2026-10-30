import { useLayoutEffect, useId, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { X } from '@phosphor-icons/react';

interface DialogProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: ReactNode;
  busy?: boolean;
  className?: string;
  bodyClassName?: string;
}

export function Dialog({
  isOpen,
  onClose,
  title,
  description,
  children,
  busy,
  className = '',
  bodyClassName = '',
}: DialogProps) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useLayoutEffect(() => {
    const element = dialog.current;
    if (!isOpen || !element) return;
    const previousFocus =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    element.showModal();
    element.querySelector<HTMLElement>('[data-autofocus]')?.focus();
    return () => {
      element.close();
      if (previousFocus?.isConnected) previousFocus.focus();
    };
  }, [isOpen]);
  if (!isOpen) return null;
  return createPortal(
    <dialog
      ref={dialog}
      aria-labelledby={titleId}
      aria-describedby={description ? `${titleId}-description` : undefined}
      className={`product-dialog ${className}`}
      onCancel={event => {
        event.preventDefault();
        if (!busy) onClose();
      }}
      onClick={event => {
        if (event.target === event.currentTarget && !busy) onClose();
      }}
    >
      <div className="product-dialog-header">
        <div>
          <h2 id={titleId}>{title}</h2>
          {description && <p id={`${titleId}-description`}>{description}</p>}
        </div>
        <button
          type="button"
          aria-label={`Close ${title.toLowerCase()}`}
          onClick={onClose}
          disabled={busy}
        >
          <X size={18} />
        </button>
      </div>
      <div className={`product-dialog-body ${bodyClassName}`}>{children}</div>
    </dialog>,
    document.body
  );
}
