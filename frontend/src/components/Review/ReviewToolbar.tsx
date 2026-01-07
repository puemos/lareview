import React from 'react';
import { ICONS } from '../../constants/icons';
import { Select } from '../Common/Select';

interface ReviewToolbarProps {
  taskTitle: string;
  taskStatus: string;
  taskDiagram?: string | null;
  activeTab: 'diff' | 'description' | 'diagram';
  onStatusChange: (status: string) => void;
  onTabChange: (tab: 'diff' | 'description' | 'diagram') => void;
  isUpdatingStatus: boolean;
}

export const ReviewToolbar: React.FC<ReviewToolbarProps> = ({
  taskTitle,
  taskStatus,
  taskDiagram,
  activeTab,
  onStatusChange,
  onTabChange,
  isUpdatingStatus,
}) => {
  const STATUS_OPTIONS = [
    {
      value: 'pending',
      label: 'Todo',
      icon: ICONS.STATUS_TODO,
      color: 'text-status-todo',
    },
    {
      value: 'in_progress',
      label: 'In Progress',
      icon: ICONS.STATUS_IN_PROGRESS,
      color: 'text-status-in_progress',
    },
    {
      value: 'done',
      label: 'Done',
      icon: ICONS.STATUS_DONE,
      color: 'text-status-done',
    },
    {
      value: 'ignored',
      label: 'Ignored',
      icon: ICONS.STATUS_IGNORED,
      color: 'text-status-ignored',
    },
  ];

  const handleStatusChange = (value: string) => {
    onStatusChange(value);
  };

  return (
    <div className="border-border bg-bg-secondary/50 z-10 border-b px-4 py-3">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-text-primary flex-1 truncate text-sm font-medium">{taskTitle}</h2>
      </div>

      <div className="flex items-center gap-2">
        <Select
          value={taskStatus}
          onChange={handleStatusChange}
          options={STATUS_OPTIONS}
          disabled={isUpdatingStatus}
        />

        <div className="bg-bg-tertiary/50 border-border/50 flex items-center rounded border">
          <button
            onClick={() => onTabChange('description')}
            className={`flex items-center gap-1.5 rounded-[3px] px-3 py-1.5 text-[11px] font-medium transition-all ${
              activeTab === 'description'
                ? 'bg-bg-primary text-text-primary shadow-sm'
                : 'text-text-disabled hover:text-text-secondary'
            }`}
          >
            <ICONS.TAB_DESCRIPTION size={12} />
            Analysis
          </button>
          <div className="bg-border/50 mx-0.5 h-3 w-px" />
          <button
            onClick={() => onTabChange('diff')}
            className={`flex items-center gap-1.5 rounded-[3px] px-3 py-1.5 text-[11px] font-medium transition-all ${
              activeTab === 'diff'
                ? 'bg-bg-primary text-text-primary shadow-sm'
                : 'text-text-disabled hover:text-text-secondary'
            }`}
          >
            <ICONS.TAB_CHANGES size={12} />
            Changes
          </button>

          {taskDiagram && (
            <>
              <div className="bg-border/50 mx-0.5 h-3 w-px" />
              <button
                onClick={() => onTabChange('diagram')}
                className={`flex items-center gap-1.5 rounded-[3px] px-3 py-1.5 text-[11px] font-medium transition-all ${
                  activeTab === 'diagram'
                    ? 'bg-bg-primary text-text-primary shadow-sm'
                    : 'text-text-disabled hover:text-text-secondary'
                }`}
              >
                <ICONS.TAB_DIAGRAM size={12} />
                Diagram
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
