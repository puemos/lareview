import React from 'react';

export const VcsSkeleton: React.FC = () => {
  return (
    <div className="space-y-6">
      <div className="bg-bg-secondary/40 border-border animate-pulse space-y-6 rounded-lg border p-6">
        <div className="border-border mb-4 flex items-center gap-4 border-b pb-4">
          <div className="bg-bg-tertiary h-10 w-10 rounded-lg" />
          <div className="space-y-2">
            <div className="bg-bg-tertiary h-4 w-24 rounded" />
            <div className="bg-bg-tertiary h-3 w-40 rounded" />
          </div>
        </div>

        <div className="grid grid-cols-[120px_1fr] items-center gap-x-8 gap-y-4">
          <div className="bg-bg-tertiary h-4 w-24 rounded" />
          <div className="bg-bg-tertiary h-5 w-32 rounded" />

          <div className="bg-bg-tertiary h-4 w-24 rounded" />
          <div className="bg-bg-tertiary h-4 w-48 rounded" />
        </div>
        <div className="pt-2">
          <div className="bg-bg-tertiary h-9 w-32 rounded" />
        </div>
      </div>
    </div>
  );
};

export const CliSkeleton: React.FC = () => {
  return (
    <div className="space-y-4">
      <div className="bg-bg-secondary/40 border-border flex animate-pulse items-center justify-between rounded-lg border p-6">
        <div className="flex items-center gap-4">
          <div className="bg-bg-tertiary border-border h-10 w-10 rounded-lg border" />
          <div className="space-y-2">
            <div className="bg-bg-tertiary h-4 w-32 rounded" />
            <div className="bg-bg-tertiary h-3 w-20 rounded" />
          </div>
        </div>
        <div className="bg-bg-tertiary h-8 w-24 rounded" />
      </div>

      <div className="border-border bg-bg-primary animate-pulse overflow-hidden rounded-lg border">
        <div className="bg-bg-secondary border-border h-8 border-b px-3 py-1.5" />
        <div className="bg-bg-primary space-y-3 p-4">
          <div className="bg-bg-tertiary h-4 w-3/4 rounded" />
          <div className="bg-bg-tertiary h-4 w-2/3 rounded" />
          <div className="bg-bg-tertiary h-4 w-1/2 rounded" />
        </div>
      </div>
    </div>
  );
};

export const EditorSkeleton: React.FC = () => {
  return (
    <div className="bg-bg-secondary/40 border-border animate-pulse rounded-lg border p-6">
      <div className="bg-bg-tertiary mb-3 h-3 w-24 rounded" />
      <div className="bg-bg-tertiary h-9 w-full max-w-xs rounded" />
      <div className="bg-bg-tertiary mt-3 h-3 w-48 rounded" />
    </div>
  );
};

export const AgentsSkeleton: React.FC = () => {
  return (
    <div className="space-y-4">
      {[1, 2].map(i => (
        <div
          key={i}
          className="bg-bg-secondary/40 border-border animate-pulse rounded-lg border p-5"
        >
          <div className="mb-4 flex items-start justify-between">
            <div className="flex items-start gap-4">
              <div className="bg-bg-tertiary border-border h-10 w-10 rounded-lg border" />
              <div>
                <div className="bg-bg-tertiary mb-2 h-4 w-32 rounded" />
                <div className="bg-bg-tertiary h-3 w-48 rounded" />
              </div>
            </div>
          </div>
          <div className="space-y-2">
            <div className="bg-bg-tertiary h-3 w-40 rounded" />
            <div className="flex gap-2">
              <div className="bg-bg-tertiary h-9 flex-1 rounded" />
              <div className="bg-bg-tertiary h-9 w-20 rounded" />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
};
