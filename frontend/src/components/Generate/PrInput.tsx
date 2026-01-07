import React from 'react';
import { GithubLogo, Spinner } from '@phosphor-icons/react';

interface PrInputProps {
  prRef: string;
  onPrRefChange: (value: string) => void;
  onFetch: () => void;
  isLoading: boolean;
  disabled: boolean;
}

export const PrInput: React.FC<PrInputProps> = ({
  prRef,
  onPrRefChange,
  onFetch,
  isLoading,
  disabled,
}) => {
  return (
    <div className="ring-border bg-bg-secondary/90 pointer-events-auto flex h-8 w-full max-w-xl min-w-[300px] flex-1 overflow-hidden rounded-md shadow-lg ring-1 shadow-black/20 backdrop-blur-md">
      <div className="text-text-disabled flex flex-shrink-0 items-center border-r border-white/5 px-2">
        <GithubLogo size={14} />
      </div>
      <input
        type="text"
        value={prRef}
        onChange={e => onPrRefChange(e.target.value)}
        placeholder="GitHub PR (e.g. owner/repo#123)..."
        className="text-text-primary placeholder-text-disabled min-w-0 flex-1 bg-transparent px-3 py-1.5 font-mono text-xs focus:outline-none"
        disabled={isLoading}
      />
      <button
        onClick={onFetch}
        disabled={!prRef.trim() || isLoading || disabled}
        className="flex min-w-[48px] flex-shrink-0 items-center justify-center gap-1 border-l border-white/5 px-3 py-1.5 text-[10px] font-medium transition-colors hover:bg-white/5 disabled:opacity-50"
      >
        {isLoading ? <Spinner size={14} className="text-text-primary animate-spin" /> : 'Fetch'}
      </button>
    </div>
  );
};
