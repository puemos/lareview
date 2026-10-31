import React, { useState, useMemo, useEffect } from 'react';
import Editor, { DiffEditor } from '@monaco-editor/react';
import { FileCode, CaretDown, CaretRight } from '@phosphor-icons/react';
import { AnimatePresence, motion } from 'framer-motion';
import { ICONS } from '../../constants/icons';
import { parseDiffLocally } from '../../utils/diffParser';
import { getLanguageFromPath } from '../../utils/languages';
import { useAppearanceStore } from '../../store/appearance';

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
  const codeFontSize = useAppearanceStore(state => state.codeFontSize);
  const [selectedFileName, setSelectedFileName] = useState<string | null>(null);
  const [isFileListExpanded, setIsFileListExpanded] = useState(true);

  const parsedDiff = useMemo(() => {
    if (viewMode !== 'diff' || !diffText.trim()) return null;
    return parseDiffLocally(diffText);
  }, [diffText, viewMode]);

  useEffect(() => {
    if (
      parsedDiff &&
      parsedDiff.files &&
      parsedDiff.files.length > 0 &&
      !parsedDiff.files.some(file => file.name === selectedFileName)
    ) {
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
        'editor.background': '#17181c',
        'editor.lineHighlightBackground': '#202227',
        'editorGutter.background': '#17181c',
        'diffEditor.insertedLineBackground': '#25433355',
        'diffEditor.removedLineBackground': '#4b293355',
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
        if (line.startsWith('@@') || line.startsWith('\\ No newline') || !line) return;
        if (line.startsWith('-')) {
          original.push(line.slice(1));
        } else if (line.startsWith('+')) {
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
    <div className="generate-diff-layout relative flex min-h-0 flex-1 overflow-hidden">
      {viewMode === 'diff' && parsedDiff && parsedDiff.files && (
        <div className="generate-diff-files border-border bg-bg-secondary/30 flex w-52 shrink-0 flex-col border-r">
          <button
            aria-expanded={isFileListExpanded}
            className="border-border hover:bg-bg-secondary flex min-h-10 items-center justify-between border-b px-3 py-2"
            onClick={() => setIsFileListExpanded(!isFileListExpanded)}
          >
            <span className="text-text-secondary text-xs font-bold tracking-wider uppercase">
              Files ({parsedDiff.files.length})
            </span>
            <div className="text-text-disabled">
              {isFileListExpanded ? <CaretDown size={14} /> : <CaretRight size={14} />}
            </div>
          </button>
          {isFileListExpanded && (
            <div className="custom-scrollbar flex-1 overflow-y-auto">
              {parsedDiff.files.map(file => (
                <button
                  key={file.name}
                  onClick={() => setSelectedFileName(file.name)}
                  aria-label={file.name}
                  aria-current={selectedFile?.name === file.name ? 'true' : undefined}
                  className={`group border-border/50 hover:bg-bg-secondary w-full border-b px-3 py-2 text-left transition-colors ${
                    selectedFileName === file.name ? 'bg-bg-secondary' : ''
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
                  {file.name.includes('/') && (
                    <div className="text-text-disabled mt-0.5 ml-5 truncate text-[0.625rem]">
                      {file.name}
                    </div>
                  )}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="relative flex min-h-0 min-w-0 flex-1 flex-col">
        {viewMode === 'raw' && !diffText.trim() && (
          <div className="pointer-events-none absolute top-14 left-16 z-10 max-w-sm pr-5">
            <h2 className="text-text-secondary text-base font-medium">Paste a unified diff</h2>
            <p className="text-text-tertiary mt-2 text-[0.8125rem] leading-6">
              Copy the output of <code className="text-text-secondary">git diff main...HEAD</code>,
              then paste it here. Use Preview to inspect the changed files.
            </p>
          </div>
        )}

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
              renderLineHighlight: 'none',
              renderOverviewRuler: false,
              minimap: { enabled: false },
              fontSize: codeFontSize,
              lineHeight: Math.round(codeFontSize * (5 / 3)),
              fontFamily: "'GeistMono', 'Monaco', monospace",
              scrollBeyondLastLine: false,
              padding: { top: 16, bottom: 16 },
              renderSideBySide: false,
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
              ariaLabel: 'Unified diff input',
              minimap: { enabled: false },
              fontSize: codeFontSize,
              lineHeight: Math.round(codeFontSize * (5 / 3)),
              fontFamily: "'GeistMono', 'Monaco', monospace",
              scrollBeyondLastLine: false,
              padding: { top: 16, bottom: 16 },
              renderLineHighlight: 'none',
              automaticLayout: true,
            }}
          />
        )}
      </div>

      <AnimatePresence>
        {validationError && (
          <motion.div
            initial={{ opacity: 0, y: 10, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 10, scale: 0.95 }}
            className="absolute right-4 bottom-16 left-4 z-20 flex justify-center"
          >
            <div className="bg-status-in_progress/5 border-status-in_progress/10 text-status-in_progress shadow-custom flex items-center gap-2 rounded-lg border px-4 py-2.5 text-xs font-medium backdrop-blur-md">
              <ICONS.ICON_WARNING size={16} weight="bold" />
              <span>{validationError}</span>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export const countAdditions = (diff: string): number => {
  return parseDiffLocally(diff).total_additions;
};

export const countDeletions = (diff: string): number => {
  return parseDiffLocally(diff).total_deletions;
};
