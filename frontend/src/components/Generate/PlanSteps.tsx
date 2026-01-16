import React from 'react';
import { CircleIcon, CircleDashedIcon, CheckCircleIcon, XCircleIcon } from '@phosphor-icons/react';
import clsx from 'clsx';

export interface PlanStep {
  content: string;
  status?: string;
}

interface PlanStepsProps {
  steps: PlanStep[];
  className?: string;
}

export const PlanSteps: React.FC<PlanStepsProps> = ({ steps, className }) => {
  if (steps.length === 0) {
    return (
      <div className="text-text-disabled flex flex-col items-center justify-center space-y-2 py-8 opacity-50">
        <CircleIcon size={24} />
        <p className="text-xs">No plan generated yet</p>
      </div>
    );
  }

  return (
    <div className={clsx('space-y-1', className)}>
      {steps.map(step => {
        const status = (step.status || 'pending').toLowerCase();
        const isDone = status === 'completed' || status === 'done';
        const isInProgress = status === 'in_progress' || status === 'inprogress';
        const isFailed = status === 'failed';

        let Icon = <CircleIcon size={16} className="text-text-disabled mt-0.5 shrink-0" />;

        if (isDone) {
          Icon = (
            <CheckCircleIcon size={16} className="mt-0.5 shrink-0 text-green-500" weight="fill" />
          );
        } else if (isInProgress) {
          Icon = (
            <CircleDashedIcon
              size={16}
              className="text-status-in_progress mt-0.5 shrink-0 animate-[spin_2000ms_linear_infinite]"
            />
          );
        } else if (isFailed) {
          Icon = (
            <XCircleIcon size={16} className="text-status-deleted mt-0.5 shrink-0" weight="fill" />
          );
        }

        return (
          <div key={step.content} className="animate-fade-in group flex items-start gap-3 text-xs">
            {Icon}
            <span
              className={clsx(
                'leading-relaxed font-medium transition-colors',
                isDone
                  ? 'text-text-primary line-through opacity-70'
                  : isInProgress
                    ? 'text-text-primary'
                    : 'text-text-secondary group-hover:text-text-primary'
              )}
            >
              {step.content}
            </span>
          </div>
        );
      })}
    </div>
  );
};
