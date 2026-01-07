import React from 'react';
import { Code, Eye } from '@phosphor-icons/react';

interface ViewModeToggleProps {
  mode: 'raw' | 'diff';
  onChange: (mode: 'raw' | 'diff') => void;
  disabled: boolean;
}

export const ViewModeToggle: React.FC<ViewModeToggleProps> = ({ mode, onChange, disabled }) => {
  return (
    <div className="bg-bg-secondary/90 ring-border pointer-events-auto flex h-8 rounded-md p-0.5 shadow-lg ring-1 shadow-black/20 backdrop-blur-md">
      <button
        onClick={() => onChange('raw')}
        className={`flex items-center gap-1.5 rounded-[4px] px-3 text-[10px] font-medium transition-all ${
          mode === 'raw'
            ? 'bg-bg-tertiary text-text-primary shadow-sm'
            : 'text-text-disabled hover:text-text-secondary'
        }`}
      >
        <Code size={14} /> Raw
      </button>
      <div className="bg-border/50 mx-0.5 my-1 w-px" />
      <button
        onClick={() => onChange('diff')}
        className={`flex items-center gap-1.5 rounded-[4px] px-3 text-[10px] font-medium transition-all ${
          mode === 'diff'
            ? 'bg-bg-tertiary text-text-primary shadow-sm'
            : 'text-text-disabled hover:text-text-secondary'
        }`}
        disabled={disabled}
      >
        <Eye size={14} /> Preview
      </button>
    </div>
  );
};
