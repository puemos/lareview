import React, { useMemo } from 'react';
import { ICONS } from '../../../constants/icons';
import type { ReviewTask, DiffFile } from '../../../types';

interface FilesHeatmapProps {
  tasks: ReviewTask[];
  diffFiles?: DiffFile[];
  onSelectFile: (fileName: string) => void;
}

type RiskLevel = 'high' | 'medium' | 'low';
const riskOrder: RiskLevel[] = ['high', 'medium', 'low'];
const riskColor: Record<RiskLevel, string> = {
  high: 'bg-risk-high',
  medium: 'bg-risk-medium',
  low: 'bg-risk-low',
};

export const FilesHeatmap: React.FC<FilesHeatmapProps> = ({
  tasks,
  diffFiles = [],
  onSelectFile,
}) => {
  const files = useMemo(() => {
    const grouped = new Map<string, { name: string; risk: RiskLevel | null; taskCount: number }>();
    for (const file of diffFiles)
      grouped.set(file.name, { name: file.name, risk: null, taskCount: 0 });
    for (const task of tasks) {
      for (const name of task.files) {
        const risk = task.risk_level || task.stats?.risk || 'low';
        const existing = grouped.get(name);
        if (existing) {
          existing.taskCount++;
          if (!existing.risk || riskOrder.indexOf(risk) < riskOrder.indexOf(existing.risk))
            existing.risk = risk;
        } else {
          grouped.set(name, { name, risk, taskCount: 1 });
        }
      }
    }
    return [...grouped.values()].sort(
      (a, b) => (a.risk ? riskOrder.indexOf(a.risk) : 3) - (b.risk ? riskOrder.indexOf(b.risk) : 3)
    );
  }, [tasks, diffFiles]);

  return (
    <section className="review-section flex max-h-80 flex-col overflow-hidden border">
      <div className="review-section-heading">
        <div className="flex items-center gap-2">
          <ICONS.ICON_FILES size={14} className="text-text-tertiary" />
          <h3 className="text-text-primary text-[13px] font-medium">Files changed</h3>
        </div>
        <span className="text-text-disabled text-xs">
          {files.length} {files.length === 1 ? 'file' : 'files'}
        </span>
      </div>
      <div className="min-h-0 overflow-y-auto">
        {files.map(file => (
          <button
            key={file.name}
            type="button"
            title={file.name}
            aria-label={file.name}
            onClick={() => onSelectFile(file.name)}
            className="border-border/30 flex min-h-12 w-full items-center gap-3 border-b px-4 py-3 text-left transition-colors outline-none last:border-0 hover:bg-white/[0.025] focus-visible:bg-white/[0.04]"
          >
            <ICONS.FILE size={15} className="text-text-tertiary shrink-0" />
            <span className="min-w-0 flex-1">
              <span className="text-text-primary block truncate font-mono text-xs">
                {file.name.split('/').pop()}
              </span>
              {file.name.includes('/') && (
                <span className="text-text-tertiary mt-0.5 block text-[11px] leading-5 break-all">
                  {file.name}
                </span>
              )}
            </span>
            <span className="text-text-tertiary flex items-center gap-1.5 text-[11px] capitalize">
              <span
                className={`${file.risk ? riskColor[file.risk] : 'bg-text-disabled'} h-1.5 w-1.5 rounded-full`}
              />
              {file.risk ?? 'Not assessed'}
            </span>
            <ICONS.ICON_ARROW_RIGHT size={12} className="text-text-disabled shrink-0" />
          </button>
        ))}
        {files.length === 0 && (
          <p className="text-text-disabled px-4 py-5 text-xs">No files with risk assessment.</p>
        )}
      </div>
    </section>
  );
};
