import React from 'react';
import type { ReviewTask } from '../../types';
import { ICONS } from '../../constants/icons';

interface TaskListProps {
  tasks: ReviewTask[];
  selectedTaskId: string | null;
  onSelectTask: (taskId: string) => void;
  isLoading: boolean;
  isFetching: boolean;
}

export const TaskList: React.FC<TaskListProps> = ({
  tasks,
  selectedTaskId,
  onSelectTask,
  isLoading,
  isFetching,
}) => {
  if (isLoading && tasks.length === 0) {
    return <TaskListSkeleton />;
  }

  if (tasks.length === 0) {
    return (
      <div className="text-text-disabled p-8 text-center text-xs opacity-50">No tasks found</div>
    );
  }

  const getRiskIcon = (risk: string) => {
    switch (risk) {
      case 'low':
        return ICONS.RISK_LOW;
      case 'medium':
        return ICONS.RISK_MEDIUM;
      case 'high':
        return ICONS.RISK_HIGH;
      default:
        return ICONS.RISK_LOW;
    }
  };

  const getRiskColor = (risk: string) => {
    switch (risk) {
      case 'low':
        return 'text-risk-low';
      case 'medium':
        return 'text-risk-medium';
      case 'high':
        return 'text-risk-high';
      default:
        return 'text-risk-low';
    }
  };

  return (
    <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto" aria-busy={isFetching}>
      {tasks.map((task: ReviewTask) => {
        const RiskIcon = getRiskIcon(task.stats.risk);
        const riskColor = getRiskColor(task.stats.risk);

        return (
          <button
            key={task.id}
            onClick={() => onSelectTask(task.id)}
            aria-current={selectedTaskId === task.id ? 'true' : undefined}
            className={`group border-border/50 hover:bg-bg-secondary/80 relative w-full border-b px-4 py-3 text-left transition-colors ${
              selectedTaskId === task.id ? 'bg-bg-secondary' : ''
            }`}
          >
            <div className="flex w-full min-w-0 items-start gap-2.5">
              <div className="mt-1 flex-shrink-0">
                <span title={`${task.stats.risk} risk`}>
                  <RiskIcon size={14} className={riskColor} />
                </span>
              </div>
              <h3
                className={`flex-1 text-[0.8125rem] leading-relaxed font-medium ${
                  selectedTaskId === task.id
                    ? 'text-text-primary'
                    : 'text-text-secondary group-hover:text-text-primary'
                } `}
              >
                {task.title}
              </h3>
              <span className="text-text-tertiary shrink-0 pt-0.5 text-[0.6875rem]">
                {task.status === 'done'
                  ? 'Done'
                  : task.status === 'in_progress'
                    ? 'In progress'
                    : task.status === 'ignored'
                      ? 'Ignored'
                      : 'To do'}
              </span>
            </div>
          </button>
        );
      })}
    </div>
  );
};

export function TaskListSkeleton() {
  return (
    <div className="custom-scrollbar flex-1 overflow-y-auto">
      {[1, 2, 3, 4, 5].map(i => (
        <div key={i} className="border-border/50 animate-pulse border-b px-4 py-3">
          <div className="flex items-start gap-2.5">
            <div className="bg-bg-tertiary mt-0.5 h-3.5 w-3.5 rounded-full" />
            <div className="flex-1 space-y-1.5">
              <div className="bg-bg-tertiary h-3 w-3/4 rounded" />
              <div className="bg-bg-tertiary h-2 w-1/2 rounded" />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
