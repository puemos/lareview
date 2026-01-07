import React from 'react';
import { ICONS } from '../../constants/icons';

interface ErrorStateProps {
  error: Error;
  onRetry?: () => void;
}

export const ErrorState: React.FC<ErrorStateProps> = ({ error, onRetry }) => {
  return (
    <div className="text-text-disabled flex h-full items-center justify-center">
      <div className="text-center">
        <div className="bg-status-ignored/10 mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-xl">
          <ICONS.ICON_WARNING size={24} className="text-status-ignored" />
        </div>
        <p className="text-sm">Failed to load</p>
        <p className="text-text-tertiary mt-1 max-w-xs text-xs">{error.message}</p>
        {onRetry && (
          <button
            onClick={onRetry}
            className="bg-bg-tertiary text-text-primary hover:bg-bg-secondary mt-3 rounded-md px-3 py-1.5 text-xs transition-colors"
          >
            Retry
          </button>
        )}
      </div>
    </div>
  );
};
