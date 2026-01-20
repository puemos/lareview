import React, { useState, useMemo } from 'react';
import { ICONS } from '../../../constants/icons';
import type { ReviewTask } from '../../../types';

interface TaskFlowProps {
  tasks: ReviewTask[];
  onSelectTask: (taskId: string) => void;
}

type RiskLevel = 'high' | 'medium' | 'low';

const riskConfig: Record<RiskLevel, { color: string; bg: string; label: string }> = {
  high: { color: 'text-risk-high', bg: 'bg-risk-high/10', label: 'HIGH' },
  medium: { color: 'text-risk-medium', bg: 'bg-risk-medium/10', label: 'MED' },
  low: { color: 'text-risk-low', bg: 'bg-risk-low/10', label: 'LOW' },
};

const statusConfig = {
  pending: { icon: ICONS.STATUS_TODO, color: 'text-text-disabled' },
  in_progress: { icon: ICONS.STATUS_IN_PROGRESS, color: 'text-status-in_progress' },
  done: { icon: ICONS.STATUS_DONE, color: 'text-status-done' },
  ignored: { icon: ICONS.STATUS_IGNORED, color: 'text-text-disabled' },
};

interface TaskGroup {
  name: string;
  tasks: ReviewTask[];
  maxRisk: RiskLevel;
}

const getRiskOrder = (risk: RiskLevel): number => {
  const order: Record<RiskLevel, number> = { high: 0, medium: 1, low: 2 };
  return order[risk];
};

interface TaskGroupSectionProps {
  group: TaskGroup;
  onSelectTask: (taskId: string) => void;
  defaultExpanded?: boolean;
}

const TaskGroupSection: React.FC<TaskGroupSectionProps> = ({
  group,
  onSelectTask,
  defaultExpanded = false,
}) => {
  const [expanded, setExpanded] = useState(defaultExpanded);

  return (
    <div className="border-border/30 border-b last:border-b-0">
      <button
        onClick={() => setExpanded(!expanded)}
        className="hover:bg-bg-tertiary/30 flex w-full items-center gap-2 px-3 py-2 text-left transition-colors"
      >
        <ICONS.CHEVRON_DOWN
          size={12}
          className={`text-text-disabled transition-transform ${expanded ? '' : '-rotate-90'}`}
        />
        <span className="text-text-primary flex-1 text-sm font-medium">{group.name}</span>
        <span className="text-text-disabled text-xs">
          {group.tasks.length} {group.tasks.length === 1 ? 'task' : 'tasks'}
        </span>
      </button>

      {expanded && (
        <div className="bg-bg-tertiary/10 border-border/20 border-t">
          {group.tasks.map(task => {
            const riskLevel = task.risk_level || task.stats?.risk || 'low';
            const risk = riskConfig[riskLevel];
            const status = statusConfig[task.status] || statusConfig.pending;
            const StatusIcon = status.icon;

            return (
              <button
                key={task.id}
                onClick={() => onSelectTask(task.id)}
                className="hover:bg-bg-tertiary/30 flex w-full items-center gap-2 px-4 py-2 text-left transition-colors"
              >
                <StatusIcon size={14} className={status.color} />
                <span className="text-text-primary min-w-0 flex-1 truncate text-sm">
                  {task.title}
                </span>
                <span
                  className={`${risk.bg} ${risk.color} flex-shrink-0 rounded px-1 py-0.5 text-[8px] font-bold`}
                >
                  {risk.label}
                </span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
};

export const TaskFlow: React.FC<TaskFlowProps> = ({ tasks, onSelectTask }) => {
  const taskGroups = useMemo(() => {
    const groupMap = new Map<string, ReviewTask[]>();

    // Group tasks by sub_flow
    tasks.forEach(task => {
      const flowName = task.sub_flow || 'General';
      const existing = groupMap.get(flowName) || [];
      groupMap.set(flowName, [...existing, task]);
    });

    // Convert to array and calculate max risk for each group
    const groups: TaskGroup[] = Array.from(groupMap.entries()).map(([name, groupTasks]) => {
      // Sort tasks within group by risk (high first)
      const sortedTasks = [...groupTasks].sort((a, b) => {
        const riskA = a.risk_level || a.stats?.risk || 'low';
        const riskB = b.risk_level || b.stats?.risk || 'low';
        return getRiskOrder(riskA) - getRiskOrder(riskB);
      });

      // Get max risk for the group
      const maxRisk = sortedTasks.reduce((max, task) => {
        const taskRisk = task.risk_level || task.stats?.risk || 'low';
        return getRiskOrder(taskRisk) < getRiskOrder(max) ? taskRisk : max;
      }, 'low' as RiskLevel);

      return { name, tasks: sortedTasks, maxRisk };
    });

    // Sort groups by max risk (high risk groups first)
    groups.sort((a, b) => getRiskOrder(a.maxRisk) - getRiskOrder(b.maxRisk));

    return groups;
  }, [tasks]);

  const completedCount = tasks.filter(t => t.status === 'done').length;

  if (tasks.length === 0) {
    return (
      <div className="bg-bg-secondary/30 border-border/50 rounded-lg border">
        <div className="border-border/50 flex items-center gap-2 border-b px-4 py-3">
          <ICONS.ICON_PLAN size={16} className="text-text-secondary" />
          <h3 className="text-text-primary text-sm font-medium">Review Tasks</h3>
        </div>
        <div className="text-text-disabled px-4 py-6 text-center text-sm">No tasks generated.</div>
      </div>
    );
  }

  return (
    <div className="bg-bg-secondary/30 border-border/50 flex max-h-80 flex-col overflow-hidden rounded-lg border">
      <div className="border-border/50 flex flex-shrink-0 items-center justify-between border-b px-4 py-3">
        <div className="flex items-center gap-2">
          <ICONS.ICON_PLAN size={16} className="text-text-secondary" />
          <h3 className="text-text-primary text-sm font-medium">Review Tasks</h3>
        </div>
        <span className="text-text-disabled text-xs">
          {tasks.length} {tasks.length === 1 ? 'task' : 'tasks'}
          {completedCount > 0 && `, ${completedCount} done`}
        </span>
      </div>
      <div className="divide-border/30 flex-1 divide-y overflow-y-auto">
        {taskGroups.map((group, idx) => (
          <TaskGroupSection
            key={group.name}
            group={group}
            onSelectTask={onSelectTask}
            defaultExpanded={idx === 0}
          />
        ))}
      </div>
    </div>
  );
};
