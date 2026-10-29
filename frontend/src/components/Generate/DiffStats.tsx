import React from 'react';

interface DiffStatsProps {
  charCount: number;
  additions: number;
  deletions: number;
}
export const DiffStats: React.FC<DiffStatsProps> = ({ charCount, additions, deletions }) => (
  <div className="border-border/50 bg-bg-secondary/60 text-text-disabled flex h-7 shrink-0 items-center justify-between border-t px-5 text-[10px] select-none">
    <span title={`${charCount.toLocaleString()} characters`}>
      {charCount ? 'Unified diff' : 'Paste a unified diff to get started'}
    </span>
    <div
      className="flex gap-3 font-mono"
      aria-label={`${additions} additions, ${deletions} deletions`}
    >
      <span className="text-status-done">+{additions}</span>
      <span className="text-red-400">−{deletions}</span>
    </div>
  </div>
);
