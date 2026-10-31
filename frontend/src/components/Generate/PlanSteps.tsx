import React from 'react';
import { CircleIcon, CircleDashedIcon, CheckCircleIcon, XCircleIcon } from '@phosphor-icons/react';
import clsx from 'clsx';

export interface PlanStep {
  content: string;
  status?: string;
}

interface PlanStepsProps {
  steps: PlanStep[];
  isRunning?: boolean;
  className?: string;
  emptyMessage?: string;
}

export const PlanSteps: React.FC<PlanStepsProps> = ({
  steps,
  isRunning = false,
  className,
  emptyMessage = 'No plan generated yet',
}) => {
  if (steps.length === 0) {
    return (
      <div
        className={clsx(
          'text-text-disabled flex min-h-[160px] flex-col items-center justify-center gap-2 text-center',
          className
        )}
      >
        <CircleIcon size={24} />
        <p className="max-w-56 text-xs leading-relaxed">{emptyMessage}</p>
      </div>
    );
  }

  return (
    <div className={clsx('space-y-1', className)}>
      {steps.map((step, index) => {
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
              className={clsx(
                'text-status-in_progress mt-0.5 shrink-0',
                isRunning && 'animate-[spin_2000ms_linear_infinite]'
              )}
            />
          );
        } else if (isFailed) {
          Icon = (
            <XCircleIcon size={16} className="text-status-deleted mt-0.5 shrink-0" weight="fill" />
          );
        }

        return (
          <div
            key={`${index}-${step.content}`}
            className="group flex items-start gap-3 py-2 text-[0.8125rem]"
          >
            {Icon}
            <span
              className={clsx(
                'leading-relaxed font-medium transition-colors',
                isDone
                  ? 'text-text-secondary'
                  : isInProgress
                    ? 'text-text-primary'
                    : 'text-text-secondary group-hover:text-text-primary'
              )}
            >
              {step.content}
            </span>
            <span className="text-text-tertiary ml-auto shrink-0 text-[0.6875rem]">
              {isDone
                ? 'Done'
                : isFailed
                  ? 'Failed'
                  : isInProgress
                    ? isRunning
                      ? 'In progress'
                      : 'Paused'
                    : 'Pending'}
            </span>
          </div>
        );
      })}
    </div>
  );
};
