import React, { useState, useMemo, useEffect } from 'react';
import Editor, { DiffEditor } from '@monaco-editor/react';
import { FileCode, CaretDown, CaretRight } from '@phosphor-icons/react';
import { parseDiffLocally } from '../../utils/diffParser';
import { getLanguageFromPath } from '../../utils/languages';

interface DiffEditorPanelProps {
  diffText: string;
  viewMode: 'raw' | 'diff';
  onDiffTextChange: (value: string) => void;
  validationError: string | null;
}

export const DiffEditorPanel: React.FC<DiffEditorPanelProps> = ({
  diffText,
  viewMode,
  onDiffTextChange,
  validationError,
}) => {
  const [selectedFileName, setSelectedFileName] = useState<string | null>(null);
  const [isFileListExpanded, setIsFileListExpanded] = useState(true);

  const parsedDiff = useMemo(() => {
    if (viewMode !== 'diff' || !diffText.trim()) return null;
    return parseDiffLocally(diffText);
  }, [diffText, viewMode]);

  useEffect(() => {
    if (parsedDiff && parsedDiff.files && parsedDiff.files.length > 0 && !selectedFileName) {
      setSelectedFileName(parsedDiff.files[0].name);
    }
  }, [parsedDiff, selectedFileName]);

  const selectedFile = useMemo(() => {
    if (!parsedDiff || !parsedDiff.files || !selectedFileName) return null;
    return parsedDiff.files.find(f => f.name === selectedFileName) || parsedDiff.files[0];
  }, [parsedDiff, selectedFileName]);

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

  const diffModels = useMemo(() => {
    if (!selectedFile) return { original: '', modified: '' };

    const original: string[] = [];
    const modified: string[] = [];

    selectedFile.hunks.forEach(hunk => {
      const content = hunk.content || '';
      content.split('\n').forEach(line => {
        if (line.startsWith('@@')) return;
        if (line.startsWith('-') && !line.startsWith('---')) {
          original.push(line.slice(1));
        } else if (line.startsWith('+') && !line.startsWith('+++')) {
          modified.push(line.slice(1));
        } else if (line.startsWith(' ')) {
          original.push(line.slice(1));
          modified.push(line.slice(1));
        } else if (
          !line.startsWith('diff ') &&
          !line.startsWith('index ') &&
          !line.startsWith('--- ') &&
          !line.startsWith('+++ ')
        ) {
          original.push(line);
          modified.push(line);
        }
      });
    });

    return {
      original: original.join('\n'),
      modified: modified.join('\n'),
    };
  }, [selectedFile]);

  const language = selectedFile ? getLanguageFromPath(selectedFile.new_path) : 'plaintext';

  return (
    <div className="relative flex flex-1 overflow-hidden pt-16">
      {viewMode === 'diff' && parsedDiff && parsedDiff.files && (
        <div className="border-border bg-bg-secondary/30 flex w-64 flex-col border-r">
          <div
            className="border-border hover:bg-bg-secondary flex cursor-pointer items-center justify-between border-b px-3 py-2"
            onClick={() => setIsFileListExpanded(!isFileListExpanded)}
          >
            <span className="text-text-secondary text-xs font-bold tracking-wider uppercase">
              Files ({parsedDiff.files.length})
            </span>
            <div className="text-text-disabled">
              {isFileListExpanded ? <CaretDown size={14} /> : <CaretRight size={14} />}
            </div>
          </div>
          {isFileListExpanded && (
            <div className="custom-scrollbar flex-1 overflow-y-auto">
              {parsedDiff.files.map(file => (
                <button
                  key={file.name}
                  onClick={() => setSelectedFileName(file.name)}
                  className={`group border-border/50 hover:bg-bg-secondary w-full border-b px-3 py-2 text-left transition-colors ${
                    selectedFileName === file.name
                      ? 'bg-bg-secondary border-l-brand border-l-2'
                      : ''
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <FileCode
                      size={14}
                      className={
                        selectedFileName === file.name ? 'text-brand' : 'text-text-tertiary'
                      }
                    />
                    <span
                      className={`flex-1 truncate text-xs ${selectedFileName === file.name ? 'text-text-primary' : 'text-text-secondary group-hover:text-text-primary'}`}
                    >
                      {file.name.split('/').pop()}
                    </span>
                  </div>
                  <div className="text-text-disabled ml-5 truncate font-mono text-[10px] opacity-60">
                    {file.name}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="flex min-w-0 flex-1 flex-col">
        {viewMode === 'diff' ? (
          <DiffEditor
            height="100%"
            theme="lareview-dark"
            onMount={handleEditorDidMount}
            original={diffModels.original}
            modified={diffModels.modified}
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
            }}
          />
        ) : (
          <Editor
            height="100%"
            defaultLanguage="diff"
            theme="lareview-dark"
            onMount={handleEditorDidMount}
            value={diffText}
            onChange={value => onDiffTextChange(value || '')}
            options={{
              minimap: { enabled: false },
              fontSize: 12,
              lineHeight: 20,
              fontFamily: "'GeistMono', 'Monaco', monospace",
              scrollBeyondLastLine: false,
              padding: { top: 16, bottom: 16 },
              renderLineHighlight: 'none',
              automaticLayout: true,
            }}
          />
        )}
      </div>

      {validationError && (
        <div className="absolute right-4 bottom-16 left-4">
          <div className="bg-status-ignored/10 border-status-ignored/20 text-status-ignored rounded-md border px-3 py-2 text-[10px]">
            {validationError}
          </div>
        </div>
      )}
    </div>
  );
};

// eslint-disable-next-line react-refresh/only-export-components
export const countAdditions = (diff: string): number => {
  return diff.split('\n').filter(line => line.startsWith('+') && !line.startsWith('+++')).length;
};

// eslint-disable-next-line react-refresh/only-export-components
export const countDeletions = (diff: string): number => {
  return diff.split('\n').filter(line => line.startsWith('-') && !line.startsWith('---')).length;
};
