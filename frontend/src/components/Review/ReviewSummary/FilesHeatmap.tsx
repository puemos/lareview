import React, { useMemo, useState } from 'react';
import { ICONS } from '../../../constants/icons';
import type { ReviewTask } from '../../../types';

interface FilesHeatmapProps {
  tasks: ReviewTask[];
  onSelectFile: (fileName: string) => void;
}

type RiskLevel = 'high' | 'medium' | 'low';

const riskConfig: Record<RiskLevel, { bg: string; hoverBg: string; borderColor: string }> = {
  high: {
    bg: 'bg-risk-high/60',
    hoverBg: 'hover:bg-risk-high/80',
    borderColor: 'border-risk-high/20',
  },
  medium: {
    bg: 'bg-risk-medium/60',
    hoverBg: 'hover:bg-risk-medium/80',
    borderColor: 'border-risk-medium/20',
  },
  low: {
    bg: 'bg-risk-low/60',
    hoverBg: 'hover:bg-risk-low/80',
    borderColor: 'border-risk-low/20',
  },
};

interface FileRiskInfo {
  name: string;
  risk: RiskLevel;
  taskCount: number;
}

interface HeatmapCellProps {
  file: FileRiskInfo;
  onClick: () => void;
  onHover: (file: FileRiskInfo | null) => void;
}

const HeatmapCell: React.FC<HeatmapCellProps> = ({ file, onClick, onHover }) => {
  const config = riskConfig[file.risk];

  return (
    <button
      onClick={onClick}
      onMouseEnter={() => onHover(file)}
      onMouseLeave={() => onHover(null)}
      className={`${config.bg} ${config.hoverBg} h-6 w-6 rounded-sm transition-colors`}
      title={`${file.name} (${file.taskCount} ${file.taskCount === 1 ? 'task' : 'tasks'})`}
    />
  );
};

export const FilesHeatmap: React.FC<FilesHeatmapProps> = ({ tasks, onSelectFile }) => {
  const [hoveredFile, setHoveredFile] = useState<FileRiskInfo | null>(null);

  const filesWithRisk = useMemo(() => {
    const fileRiskMap = new Map<string, { risk: RiskLevel; taskCount: number }>();

    tasks.forEach(task => {
      task.files.forEach(file => {
        const existing = fileRiskMap.get(file);
        const taskRisk = task.risk_level || task.stats?.risk || 'low';

        if (!existing) {
          fileRiskMap.set(file, { risk: taskRisk, taskCount: 1 });
        } else {
          // Keep highest risk
          const riskOrder: RiskLevel[] = ['low', 'medium', 'high'];
          const currentIdx = riskOrder.indexOf(existing.risk);
          const newIdx = riskOrder.indexOf(taskRisk);
          fileRiskMap.set(file, {
            risk: newIdx > currentIdx ? taskRisk : existing.risk,
            taskCount: existing.taskCount + 1,
          });
        }
      });
    });

    // Convert to array and sort by risk (high first)
    return Array.from(fileRiskMap.entries())
      .map(([name, info]) => ({ name, ...info }))
      .sort((a, b) => {
        const riskOrder: RiskLevel[] = ['high', 'medium', 'low'];
        return riskOrder.indexOf(a.risk) - riskOrder.indexOf(b.risk);
      });
  }, [tasks]);

  const highCount = filesWithRisk.filter(f => f.risk === 'high').length;
  const mediumCount = filesWithRisk.filter(f => f.risk === 'medium').length;
  const lowCount = filesWithRisk.filter(f => f.risk === 'low').length;

  if (filesWithRisk.length === 0) {
    return (
      <div className="bg-bg-secondary/30 border-border/50 rounded-lg border">
        <div className="border-border/50 flex items-center gap-2 border-b px-4 py-3">
          <ICONS.ICON_FILES size={16} className="text-text-secondary" />
          <h3 className="text-text-primary text-sm font-medium">Files Changed</h3>
        </div>
        <div className="text-text-disabled px-4 py-6 text-center text-sm">
          No files with risk assessment.
        </div>
      </div>
    );
  }

  return (
    <div className="bg-bg-secondary/30 border-border/50 flex max-h-80 flex-col overflow-hidden rounded-lg border">
      <div className="border-border/50 flex flex-shrink-0 items-center justify-between border-b px-4 py-3">
        <div className="flex items-center gap-2">
          <ICONS.ICON_FILES size={16} className="text-text-secondary" />
          <h3 className="text-text-primary text-sm font-medium">Files Changed</h3>
        </div>
        <span className="text-text-disabled text-xs">{filesWithRisk.length} files</span>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {/* Heatmap grid */}
        <div className="mb-4 flex flex-wrap gap-1">
          {filesWithRisk.map(file => (
            <HeatmapCell
              key={file.name}
              file={file}
              onClick={() => onSelectFile(file.name)}
              onHover={setHoveredFile}
            />
          ))}
        </div>

        {/* Legend */}
        <div className="flex items-center justify-center gap-4 text-[10px]">
          {highCount > 0 && (
            <div className="flex items-center gap-1">
              <div className="bg-risk-high/60 h-3 w-3 rounded-sm" />
              <span className="text-text-secondary">HIGH ({highCount})</span>
            </div>
          )}
          {mediumCount > 0 && (
            <div className="flex items-center gap-1">
              <div className="bg-risk-medium/60 h-3 w-3 rounded-sm" />
              <span className="text-text-secondary">MEDIUM ({mediumCount})</span>
            </div>
          )}
          {lowCount > 0 && (
            <div className="flex items-center gap-1">
              <div className="bg-risk-low/60 h-3 w-3 rounded-sm" />
              <span className="text-text-secondary">LOW ({lowCount})</span>
            </div>
          )}
        </div>
      </div>

      {/* Hover info bar */}
      <div className="border-border/50 flex h-8 flex-shrink-0 items-center border-t px-4">
        {hoveredFile ? (
          <>
            <span className="text-text-primary min-w-0 flex-1 truncate font-mono text-xs">
              {hoveredFile.name}
            </span>
            <span className="text-text-secondary ml-2 text-xs">
              {hoveredFile.taskCount} {hoveredFile.taskCount === 1 ? 'task' : 'tasks'}
            </span>
          </>
        ) : (
          <span className="text-text-disabled text-xs">Hover over a file to see details</span>
        )}
      </div>
    </div>
  );
};
