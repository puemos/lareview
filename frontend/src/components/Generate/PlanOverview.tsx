import React from 'react';
import { ListChecks } from '@phosphor-icons/react';
import { PlanSteps, PlanStep } from './PlanSteps';

interface PlanOverviewProps {
  items: PlanStep[];
  isRunning?: boolean;
  emptyMessage?: string;
}

export const PlanOverview: React.FC<PlanOverviewProps> = ({
  items,
  isRunning = false,
  emptyMessage,
}) => {
  return (
    <div className="border-border bg-bg-primary/30 flex min-h-0 flex-1 flex-col">
      <div className="border-border bg-bg-secondary flex items-center justify-between border-b px-4 py-2">
        <h2 className="text-text-secondary flex items-center gap-2 text-xs font-medium">
          <ListChecks size={14} />
          Plan
        </h2>
        <span className="bg-bg-tertiary text-text-secondary rounded px-1.5 text-[11px]">
          {items.filter(item => item.status === 'completed' || item.status === 'done').length} of{' '}
          {items.length} done
        </span>
      </div>
      <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto p-4">
        <PlanSteps
          isRunning={isRunning}
          steps={items}
          className="h-full"
          emptyMessage={emptyMessage}
        />
      </div>
    </div>
  );
};
