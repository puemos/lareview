import { forwardRef, type ComponentPropsWithoutRef, type ReactNode } from 'react';
import { reviewHeaderControlClass } from './reviewHeaderStyles';

interface ReviewHeaderBadgeProps extends ComponentPropsWithoutRef<'button'> {
  icon: ReactNode;
}

// Usage and quality results deliberately share the same surface, icon slot, and metrics.
export const ReviewHeaderBadge = forwardRef<HTMLButtonElement, ReviewHeaderBadgeProps>(
  function ReviewHeaderBadge({ icon, children, className = '', ...props }, ref) {
    return (
      <button
        ref={ref}
        type="button"
        className={`${reviewHeaderControlClass} px-3 text-sm font-medium hover:brightness-110 ${className}`}
        {...props}
      >
        <span aria-hidden="true" className="flex size-4 shrink-0 items-center justify-center">
          {icon}
        </span>
        <span className="tabular-nums">{children}</span>
      </button>
    );
  }
);
