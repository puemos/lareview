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
  if (isLoading || isFetching) {
    return <TaskListSkeleton />;
  }

  if (tasks.length === 0) {
    return (
      <div className="text-text-disabled p-8 text-center text-xs opacity-50">No tasks found</div>
    );
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'pending':
        return ICONS.STATUS_TODO;
      case 'in_progress':
        return ICONS.STATUS_IN_PROGRESS;
      case 'done':
        return ICONS.STATUS_DONE;
      case 'ignored':
        return ICONS.STATUS_IGNORED;
      default:
        return ICONS.STATUS_TODO;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'pending':
        return 'text-status-todo';
      case 'in_progress':
        return 'text-status-in_progress';
      case 'done':
        return 'text-status-done';
      case 'ignored':
        return 'text-status-ignored';
      default:
        return 'text-status-todo';
    }
  };

  return (
    <div className="custom-scrollbar flex-1 overflow-y-auto">
      {tasks.map((task: ReviewTask) => {
        const StatusIcon = getStatusIcon(task.status);
        const statusColor = getStatusColor(task.status);

        return (
          <button
            key={task.id}
            onClick={() => onSelectTask(task.id)}
            className={`group border-border/50 hover:bg-bg-secondary/80 relative w-full border-b px-4 py-3 text-left transition-all ${
              selectedTaskId === task.id ? 'bg-bg-secondary shadow-inner' : ''
            }`}
          >
            {selectedTaskId === task.id && (
              <div className="bg-brand absolute top-0 bottom-0 left-0 w-[2px]" />
            )}
            <div className="flex w-full min-w-0 items-center gap-2.5">
              <div className="flex-shrink-0">
                <StatusIcon size={14} className={statusColor} />
              </div>
              <h3
                className={`flex-1 truncate text-xs leading-relaxed font-medium ${
                  selectedTaskId === task.id
                    ? 'text-text-primary'
                    : 'text-text-secondary group-hover:text-text-primary'
                } ${task.status === 'done' ? 'text-text-disabled line-through opacity-50' : ''}`}
              >
                {task.title}
              </h3>
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
