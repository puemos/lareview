import React from 'react';

const CardSkeleton: React.FC<{ height?: string }> = ({ height = 'h-48' }) => (
  <div className={`bg-bg-tertiary/30 border-border/50 animate-pulse rounded-lg border p-4 ${height}`}>
    <div className="bg-bg-tertiary mb-4 h-3 w-24 rounded uppercase" />
    <div className="space-y-3">
      <div className="bg-bg-tertiary h-4 w-full rounded" />
      <div className="bg-bg-tertiary h-4 w-5/6 rounded" />
      <div className="bg-bg-tertiary h-4 w-4/6 rounded" />
    </div>
  </div>
);

export const SummarySkeleton: React.FC = () => {
  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header Skeleton */}
      <div className="border-border bg-bg-secondary/50 flex items-center justify-between border-b px-6 py-4">
        <div className="space-y-2">
          <div className="bg-bg-tertiary animate-pulse h-3 w-16 rounded" />
          <div className="bg-bg-tertiary animate-pulse h-6 w-48 rounded" />
        </div>
        <div className="bg-brand/30 animate-pulse h-10 w-32 rounded-md" />
      </div>

      {/* Content Skeleton */}
      <div className="flex-1 space-y-6 overflow-auto p-6">
        {/* Summary Card */}
        <CardSkeleton height="h-32" />

        {/* Grid for Task Flow and Files Heatmap */}
        <div className="grid grid-cols-2 items-start gap-6">
          <CardSkeleton />
          <CardSkeleton />
        </div>

        {/* Grid for Issue Checklist and Key Feedback */}
        <div className="grid grid-cols-2 items-start gap-6">
          <CardSkeleton height="h-64" />
          <CardSkeleton height="h-64" />
        </div>
      </div>
    </div>
  );
};
