import { WarningCircle, ArrowClockwise } from '@phosphor-icons/react';

interface InlineErrorProps {
  title: string;
  message?: string;
  onRetry?: () => void;
  isRetrying?: boolean;
}

export function InlineError({ title, message, onRetry, isRetrying }: InlineErrorProps) {
  return (
    <div
      role="alert"
      className="flex flex-wrap items-start gap-3 rounded-lg border border-red-400/20 bg-red-400/5 p-4"
    >
      <WarningCircle size={18} className="mt-0.5 shrink-0 text-red-400" />
      <div className="min-w-0 flex-1">
        <p className="text-text-primary text-sm font-medium">{title}</p>
        {message && (
          <p className="text-text-secondary mt-1 text-xs leading-5 break-words">{message}</p>
        )}
      </div>
      {onRetry && (
        <button
          type="button"
          onClick={onRetry}
          disabled={isRetrying}
          className="text-text-secondary hover:text-text-primary border-border flex h-8 items-center gap-2 rounded-md border px-3 text-xs"
        >
          <ArrowClockwise size={13} className={isRetrying ? 'animate-spin' : ''} />
          {isRetrying ? 'Retrying…' : 'Try again'}
        </button>
      )}
    </div>
  );
}
