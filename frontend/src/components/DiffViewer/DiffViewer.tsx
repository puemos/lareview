import React, { useState, useMemo } from 'react';
import { Chat, CaretDown, CaretRight, FileCode, ArrowSquareOut } from '@phosphor-icons/react';
import { DiffEditor } from '@monaco-editor/react';
import type { DiffFile } from '../../types';
import { useTauri } from '../../hooks/useTauri';
import { getLanguageFromPath } from '../../utils/languages';

interface DiffViewerProps {
  files: DiffFile[];
  selectedFile: DiffFile | null;
  onSelectFile: (file: DiffFile | null) => void;
  highlightedHunks?: Array<{
    file: string;
    oldStart: number;
    oldLines: number;
    newStart: number;
    newLines: number;
  }>;
  viewMode?: 'unified' | 'split';
}

export const DiffViewer: React.FC<DiffViewerProps> = ({
  files,
  selectedFile,
  onSelectFile,
  highlightedHunks = [],
  viewMode = 'split',
}) => {
  return (
    <div className="bg-bg-primary flex h-full">
      <FileList files={files} selectedFile={selectedFile} onSelectFile={onSelectFile} />
      <div className="flex flex-1 flex-col">
        {selectedFile ? (
          <DiffContent
            file={selectedFile}
            highlightedHunks={highlightedHunks.filter(
              h => h.file === selectedFile.name || h.file === selectedFile.new_path
            )}
            viewMode={viewMode}
          />
        ) : (
          <div className="text-text-disabled flex flex-1 items-center justify-center">
            <div className="text-center">
              <Chat size={48} className="mx-auto mb-3 opacity-50" />
              <p>Select a file to view diff</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

interface FileListProps {
  files: DiffFile[];
  selectedFile: DiffFile | null;
  onSelectFile: (file: DiffFile | null) => void;
}

const FileList: React.FC<FileListProps> = ({ files, selectedFile, onSelectFile }) => {
  const [expanded, setExpanded] = useState(true);

  return (
    <div className="border-border bg-bg-secondary/30 flex w-64 flex-col border-r">
      <div
        className="border-border hover:bg-bg-secondary flex cursor-pointer items-center justify-between border-b px-3 py-2"
        onClick={() => setExpanded(!expanded)}
      >
        <span className="text-text-secondary text-xs font-bold tracking-wider uppercase">
          Changed Files ({files.length})
        </span>
        <div className="text-text-disabled">
          {expanded ? <CaretDown size={14} /> : <CaretRight size={14} />}
        </div>
      </div>
      {expanded && (
        <div className="custom-scrollbar flex-1 overflow-y-auto">
          {files.map(file => (
            <FileListItem
              key={file.name || file.new_path}
              file={file}
              isSelected={
                selectedFile?.name === file.name || selectedFile?.new_path === file.new_path
              }
              onClick={() => onSelectFile(file)}
            />
          ))}
        </div>
      )}
    </div>
  );
};

interface FileListItemProps {
  file: DiffFile;
  isSelected: boolean;
  onClick: () => void;
}

const FileListItem: React.FC<FileListItemProps> = ({ file, isSelected, onClick }) => {
  const additions = file.hunks.reduce((sum, h) => sum + h.new_lines, 0);
  const deletions = file.hunks.reduce((sum, h) => sum + h.old_lines, 0);
  const path = file.name || file.new_path || 'unknown';

  return (
    <button
      onClick={onClick}
      className={`group border-border/50 hover:bg-bg-secondary w-full border-b px-3 py-2 text-left transition-colors ${
        isSelected ? 'bg-bg-secondary border-l-brand border-l-2' : ''
      }`}
    >
      <div className="flex items-center gap-2">
        <FileCode size={14} className={isSelected ? 'text-brand' : 'text-text-tertiary'} />
        <span
          className={`flex-1 truncate text-xs ${isSelected ? 'text-text-primary' : 'text-text-secondary group-hover:text-text-primary'}`}
        >
          {path.split('/').pop()}
        </span>
        <span className="text-status-added text-[10px]">+{additions}</span>
        <span className="text-status-deleted text-[10px]">-{deletions}</span>
      </div>
      <div className="text-text-disabled ml-5 truncate font-mono text-[10px] opacity-60">
        {path}
      </div>
    </button>
  );
};

interface DiffContentProps {
  file: DiffFile;
  highlightedHunks: Array<{
    oldStart: number;
    oldLines: number;
    newStart: number;
    newLines: number;
  }>;
  viewMode: 'unified' | 'split';
}

const DiffContent: React.FC<DiffContentProps> = ({ file, highlightedHunks }) => {
  const { openInEditor } = useTauri();
  const path = file.name || file.new_path || 'unknown';
  const language = getLanguageFromPath(path);

  const handleOpenInEditor = async () => {
    try {
      const lineNumber = highlightedHunks.length > 0 ? highlightedHunks[0].newStart : 1;
      await openInEditor(path, lineNumber);
    } catch (error) {
      console.error('Failed to open file in editor:', error);
    }
  };

  const { original, modified } = useMemo(() => {
    const originalLines: string[] = [];
    const modifiedLines: string[] = [];

    file.hunks.forEach(hunk => {
      const content = hunk.content || '';
      const contentLines = content.split('\n');

      for (let i = 0; i < contentLines.length; i++) {
        const line = contentLines[i];

        if (line.startsWith('@@')) {
          continue;
        }

        if (line.startsWith('-') && !line.startsWith('---')) {
          originalLines.push(line.slice(1));
          modifiedLines.push('');
        } else if (line.startsWith('+') && !line.startsWith('+++')) {
          originalLines.push('');
          modifiedLines.push(line.slice(1));
        } else {
          originalLines.push(line);
          modifiedLines.push(line);
        }
      }
    });

    return {
      original: originalLines.join('\n'),
      modified: modifiedLines.join('\n'),
    };
  }, [file.hunks]);

  const handleEditorDidMount = (_editor: unknown, monaco: typeof import('monaco-editor')) => {
    monaco.editor.defineTheme('lareview-dark', {
      base: 'vs-dark',
      inherit: true,
      rules: [],
      colors: {
        'editor.background': '#1e1e2e',
        'editor.lineHighlightBackground': '#313244',
      },
    });
    monaco.editor.setTheme('lareview-dark');
  };

  return (
    <div className="bg-bg-primary flex flex-1 flex-col overflow-hidden">
      <div className="border-border bg-bg-secondary/50 flex items-center justify-between border-b px-4 py-2">
        <div className="flex items-center gap-3">
          <span className="text-text-primary font-mono text-xs">{path}</span>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleOpenInEditor}
            className="bg-bg-tertiary hover:bg-bg-secondary text-text-secondary hover:text-text-primary border-border flex items-center gap-1.5 rounded border px-2 py-1 transition-colors"
            title="Open in External Editor"
          >
            <ArrowSquareOut size={12} />
            <span className="text-[10px] font-medium">Open in Editor</span>
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-hidden">
        <DiffEditor
          height="100%"
          language={language}
          theme="lareview-dark"
          original={original}
          modified={modified}
          onMount={handleEditorDidMount}
          originalLanguage={language}
          modifiedLanguage={language}
          options={{
            readOnly: true,
            minimap: { enabled: false },
            fontSize: 12,
            lineHeight: 20,
            fontFamily: "'GeistMono', 'Monaco', monospace",
            scrollBeyondLastLine: false,
            padding: { top: 16, bottom: 16 },
            renderSideBySide: true,
            automaticLayout: true,
            originalEditable: false,
            renderLineHighlight: 'none',
            diffWordWrap: 'off' as const,
            ignoreTrimWhitespace: false,
            codeLens: false,
            folding: true,
            glyphMargin: false,
            lineNumbers: 'on' as const,
            renderIndicators: false,
          }}
        />
      </div>
    </div>
  );
};
