import React, { useState } from 'react';
import { ICONS } from '../../../constants/icons';

interface UncoveredFilesProps {
  uncoveredFiles: string[];
  onSelectFile: (fileName: string) => void;
}

export const UncoveredFiles: React.FC<UncoveredFilesProps> = ({
  uncoveredFiles,
  onSelectFile,
}) => {
  const [expanded, setExpanded] = useState(false);

  if (uncoveredFiles.length === 0) {
    return null;
  }

  return (
    <div className="bg-bg-secondary/30 border-border/50 rounded-lg border">
      <button
        onClick={() => setExpanded(!expanded)}
        className="hover:bg-bg-tertiary/30 flex w-full items-center justify-between px-4 py-3 transition-colors"
      >
        <div className="flex items-center gap-2">
          <ICONS.FILE size={16} className="text-text-disabled" />
          <h3 className="text-text-primary text-sm font-medium">Uncovered Files</h3>
          <span className="bg-bg-tertiary text-text-secondary rounded-full px-2 py-0.5 text-[10px] font-medium">
            {uncoveredFiles.length}
          </span>
        </div>
        <ICONS.CHEVRON_DOWN
          size={14}
          className={`text-text-disabled transition-transform ${expanded ? 'rotate-180' : ''}`}
        />
      </button>

      {expanded && (
        <div className="border-border/50 max-h-48 overflow-y-auto border-t">
          {uncoveredFiles.map(file => (
            <button
              key={file}
              onClick={() => onSelectFile(file)}
              className="hover:bg-bg-tertiary/30 border-border/20 flex w-full items-center gap-2 border-b px-4 py-2 text-left transition-colors last:border-b-0"
            >
              <ICONS.FILE size={12} className="text-text-disabled flex-shrink-0" />
              <span className="text-text-secondary min-w-0 flex-1 truncate font-mono text-xs">
                {file}
              </span>
            </button>
          ))}
        </div>
      )}

      {!expanded && (
        <div className="border-border/50 border-t px-4 py-2">
          <p className="text-text-disabled text-xs">
            These files were changed but not included in any review task.
          </p>
        </div>
      )}
    </div>
  );
};
