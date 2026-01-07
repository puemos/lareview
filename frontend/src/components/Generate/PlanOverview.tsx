import React from 'react';
import { ListChecks, CaretDown, CaretUp } from '@phosphor-icons/react';
import clsx from 'clsx';

interface PlanEntry {
  content: string;
  status?: string;
}

interface PlanOverviewProps {
  items: PlanEntry[];
  isExpanded: boolean;
  onToggle: () => void;
}

export const PlanOverview: React.FC<PlanOverviewProps> = ({ items, isExpanded, onToggle }) => {
  return (
    <div className="border-border bg-bg-primary/30 flex min-h-0 flex-col border-b">
      <div
        role="button"
        tabIndex={0}
        onClick={onToggle}
        onKeyDown={e => e.key === 'Enter' && onToggle()}
        className="border-border bg-bg-secondary hover:bg-bg-tertiary/50 flex cursor-pointer items-center justify-between border-b px-4 py-2 transition-colors outline-none"
      >
        <h2 className="text-text-disabled flex items-center gap-2 text-[10px] font-bold tracking-wider uppercase">
          <ListChecks size={14} />
          Plan
        </h2>
        <div className="flex items-center gap-2">
          <span className="bg-bg-tertiary text-text-secondary rounded px-1.5 text-[10px]">
            {items.length}
          </span>
          {isExpanded ? (
            <CaretUp size={12} className="text-text-disabled" />
          ) : (
            <CaretDown size={12} className="text-text-disabled" />
          )}
        </div>
      </div>
      <div
        className={clsx(
          'grid transition-[grid-template-rows] duration-200 ease-out',
          isExpanded ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]'
        )}
      >
        <div className="overflow-hidden">
          <div className="custom-scrollbar flex-1 overflow-y-auto p-4">
            <PlanSteps steps={items} />
          </div>
        </div>
      </div>
    </div>
  );
};

interface PlanStepsProps {
  steps: PlanEntry[];
}

const PlanSteps: React.FC<PlanStepsProps> = ({ steps }) => {
  if (steps.length === 0) {
    return (
      <div className="text-text-disabled py-8 text-center text-xs opacity-50">
        No plan items yet
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {steps.map((step, index) => (
        <div
          key={index}
          className="bg-bg-secondary/50 hover:bg-bg-secondary flex items-start gap-2 rounded-md p-2 transition-colors"
        >
          <div className="bg-brand mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full" />
          <span className="text-text-secondary text-xs leading-relaxed">{step.content}</span>
        </div>
      ))}
    </div>
  );
};
