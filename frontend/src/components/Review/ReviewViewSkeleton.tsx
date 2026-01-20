import React from 'react';
import { SummarySkeleton } from './SummarySkeleton';
import { TaskListSkeleton } from './TaskList';
import { DiffSkeleton } from './DiffSkeleton';

const TabButtonSkeleton = () => (
  <div className="bg-bg-primary/50 flex flex-1 animate-pulse items-center justify-center gap-1.5 rounded-[3px] py-1.5 shadow-sm">
    <div className="bg-bg-tertiary h-3 w-3 rounded-full" />
    <div className="bg-bg-tertiary h-3 w-10 rounded" />
  </div>
);

interface ReviewViewSkeletonProps {
  mode?: 'review' | 'summary';
}

export const ReviewViewSkeleton: React.FC<ReviewViewSkeletonProps> = ({ mode = 'review' }) => {
  if (mode === 'summary') {
    return <SummarySkeleton />;
  }

  return (
    <div className="bg-bg-primary flex h-full">
      {/* Sidebar Skeleton */}
      <div className="border-border bg-bg-secondary/30 flex w-[300px] flex-col border-r">
        <div className="border-border bg-bg-secondary/50 border-b p-3">
          {/* Back to Summary placeholder */}
          <div className="mb-3 flex animate-pulse items-center gap-1.5">
            <div className="bg-bg-tertiary h-3 w-3 rounded-full" />
            <div className="bg-bg-tertiary h-2 w-16 rounded" />
          </div>

          <div className="mb-3 flex gap-2">
            <div className="bg-bg-tertiary animate-pulse h-8 flex-1 rounded py-1.5" />
            <div className="bg-bg-tertiary animate-pulse h-8 flex-1 rounded py-1.5" />
          </div>

          <div className="bg-bg-tertiary border-border/50 flex gap-0.5 rounded-md border p-0.5">
            <TabButtonSkeleton />
            <TabButtonSkeleton />
          </div>
        </div>

        <TaskListSkeleton />
      </div>

      {/* Main Content Skeleton */}
      <div className="bg-bg-primary relative flex min-w-0 flex-1 flex-col">
        <DiffSkeleton />
      </div>
    </div>
  );
};
