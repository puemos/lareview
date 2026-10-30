import React from 'react';
import { Code, Eye } from '@phosphor-icons/react';

interface ViewModeToggleProps {
  mode: 'raw' | 'diff';
  onChange: (mode: 'raw' | 'diff') => void;
  disabled: boolean;
}

export const ViewModeToggle: React.FC<ViewModeToggleProps> = ({ mode, onChange, disabled }) => (
  <div
    className="border-border/50 bg-bg-secondary/50 pointer-events-auto inline-flex h-8 shrink-0 items-center gap-0.5 rounded-md border p-0.5"
    role="group"
    aria-label="Diff view"
  >
    {(['raw', 'diff'] as const).map(value => (
      <button
        key={value}
        type="button"
        onClick={() => onChange(value)}
        disabled={value === 'diff' && disabled}
        aria-pressed={mode === value}
        className={`focus-visible:outline-brand inline-flex h-6 items-center gap-1.5 rounded px-2.5 text-[11px] font-medium transition-colors focus-visible:outline-2 ${mode === value ? 'text-text-primary bg-white/[0.07]' : 'text-text-disabled hover:text-text-secondary'}`}
      >
        {value === 'raw' ? <Code size={13} /> : <Eye size={13} />}
        {value === 'raw' ? 'Raw' : 'Preview'}
      </button>
    ))}
  </div>
);
