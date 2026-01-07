import React from 'react';

interface DiffStatsProps {
  charCount: number;
  additions: number;
  deletions: number;
}

export const DiffStats: React.FC<DiffStatsProps> = ({ charCount, additions, deletions }) => {
  return (
    <div className="border-border bg-bg-secondary text-text-disabled flex justify-between border-t px-4 py-1.5 font-mono text-[10px] select-none">
      <span>{charCount} chars</span>
      <div className="flex gap-3">
        <span className="text-green-400">+{additions}</span>
        <span className="text-red-400">-{deletions}</span>
      </div>
    </div>
  );
};
