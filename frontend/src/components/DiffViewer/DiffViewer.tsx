import React, { useState, useMemo } from 'react';
import { Chat, CaretDown, CaretRight, FileCode, ArrowSquareOut } from '@phosphor-icons/react';
import { DiffEditor } from '@monaco-editor/react';
import type { DiffFile } from '../../types';
import { useTauri } from '../../hooks/useTauri';
import { getLanguageFromPath } from '../../utils/languages';
import { InlineError } from '../Common/InlineError';
import { countDiffChanges } from '../../lib/diff-stats';
import { GutterMenu } from './GutterMenu';
import { useAppearanceStore } from '../../store/appearance';
import type { CodeLocation } from '../../lib/code-references';

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
  onAddFeedback?: (file: DiffFile, line: number, side: 'old' | 'new') => void;
  repoRoot?: string | null;
  focusLocation?: Pick<CodeLocation, 'line' | 'side'>;
  showFileList?: boolean;
}

export const DiffViewer: React.FC<DiffViewerProps> = ({
  files,
  selectedFile,
  onSelectFile,
  highlightedHunks = [],
  viewMode = 'unified',
  onAddFeedback,
  repoRoot,
  focusLocation,
  showFileList = true,
}) => {
  return (
    <div className="review-diff-layout bg-bg-primary flex min-h-0 min-w-0 flex-1">
      {showFileList && (
        <FileList files={files} selectedFile={selectedFile} onSelectFile={onSelectFile} />
      )}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        {selectedFile ? (
          <DiffContent
            key={selectedFile.name}
            file={selectedFile}
            highlightedHunks={highlightedHunks.filter(
              h => h.file === selectedFile.name || h.file === selectedFile.new_path
            )}
            viewMode={viewMode}
            onAddFeedback={onAddFeedback}
            repoRoot={repoRoot}
            focusLocation={focusLocation}
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
    <div className="review-diff-files border-border bg-bg-secondary/30 flex w-56 shrink-0 flex-col border-r">
      <button
        aria-expanded={expanded}
        className="border-border hover:bg-bg-secondary flex min-h-10 items-center justify-between border-b px-3 py-2"
        onClick={() => setExpanded(!expanded)}
      >
        <span className="text-text-secondary text-xs font-bold tracking-wider uppercase">
          Changed Files ({files.length})
        </span>
        <div className="text-text-disabled">
          {expanded ? <CaretDown size={14} /> : <CaretRight size={14} />}
        </div>
      </button>
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
  const { additions, deletions } = countDiffChanges([file]);
  const path = file.name || file.new_path || 'unknown';

  return (
    <button
      onClick={onClick}
      aria-current={isSelected ? 'true' : undefined}
      aria-label={path}
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
        <span className="text-status-done text-xs">+{additions}</span>
        <span className="text-xs text-red-400">-{deletions}</span>
      </div>
      <div className="text-text-tertiary ml-5 truncate font-mono text-[0.6875rem]">{path}</div>
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
  onAddFeedback?: (file: DiffFile, line: number, side: 'old' | 'new') => void;
  repoRoot?: string | null;
  focusLocation?: Pick<CodeLocation, 'line' | 'side'>;
}

const DiffContent: React.FC<DiffContentProps> = ({
  file,
  highlightedHunks,
  viewMode,
  onAddFeedback,
  repoRoot,
  focusLocation,
}) => {
  const { openInEditor } = useTauri();
  const codeFontSize = useAppearanceStore(state => state.codeFontSize);
  const [editorError, setEditorError] = useState<string | null>(null);
  const [displayMode, setDisplayMode] = useState<'unified' | 'split'>(
    focusLocation?.side === 'old' ? 'split' : viewMode
  );
  const path = file.name || file.new_path || 'unknown';
  const language = getLanguageFromPath(path);
  // Ref to store diff editor instance
  const diffEditorRef = React.useRef<import('monaco-editor').editor.IDiffEditor | null>(null);

  // Auto-detect new files (all additions, no deletions) or purely deleted files
  // If it's a new file, we might want to default to 'unified' or just ensure side-by-side looks good.
  // Actually, for new files, side-by-side with original empty is standard, but let's ensure the minimap helps.

  const handleOpenInEditor = async () => {
    setEditorError(null);
    try {
      const lineNumber =
        focusLocation?.line ?? (highlightedHunks.length > 0 ? highlightedHunks[0].newStart : 1);
      await openInEditor(path, lineNumber, repoRoot || undefined);
    } catch (error) {
      setEditorError(String(error));
    }
  };

  const [menuState, setMenuState] = useState<{
    x: number;
    y: number;
    file: DiffFile;
    line: number;
    side: 'old' | 'new';
  } | null>(null);

  // Ref to track menu state inside stable callbacks
  const menuStateRef = React.useRef(menuState);
  React.useEffect(() => {
    menuStateRef.current = menuState;
  }, [menuState]);

  const closeMenu = () => setMenuState(null);

  const handleMenuAddFeedback = () => {
    if (menuState && onAddFeedback) {
      onAddFeedback(menuState.file, menuState.line, menuState.side);
    }
  };

  const handleMenuOpenInEditor = async () => {
    if (menuState) {
      try {
        await openInEditor(path, menuState.line, repoRoot || undefined);
      } catch (error) {
        setEditorError(String(error));
      }
    }
  };

  // Build original and modified content strings along with line number mappings
  // originalLineMap[i] = actual file line number for Monaco line i+1
  const { original, modified, originalLineMap, modifiedLineMap } = useMemo(() => {
    const originalLines: string[] = [];
    const modifiedLines: string[] = [];
    const origLineMap: number[] = [];
    const modLineMap: number[] = [];

    file.hunks.forEach(hunk => {
      const content = hunk.content || '';
      const contentLines = content.split('\n');
      let origLine = hunk.old_start;
      let modLine = hunk.new_start;

      for (let i = 0; i < contentLines.length; i++) {
        const line = contentLines[i];

        if (line.startsWith('@@') || line.startsWith('\\ No newline') || !line) {
          continue;
        }

        if (line.startsWith('-')) {
          // Deletion: add to original only
          originalLines.push(line.slice(1));
          origLineMap.push(origLine);
          origLine++;
        } else if (line.startsWith('+')) {
          // Addition: add to modified only
          modifiedLines.push(line.slice(1));
          modLineMap.push(modLine);
          modLine++;
        } else {
          // Context line: add to both (strip leading space if present)
          const contextLine = line.startsWith(' ') ? line.slice(1) : line;
          originalLines.push(contextLine);
          modifiedLines.push(contextLine);
          origLineMap.push(origLine);
          modLineMap.push(modLine);
          origLine++;
          modLine++;
        }
      }
    });

    return {
      original: originalLines.join('\n'),
      modified: modifiedLines.join('\n'),
      originalLineMap: origLineMap,
      modifiedLineMap: modLineMap,
    };
  }, [file.hunks]);

  // Ensure line numbers are updated when maps or editor changes
  React.useEffect(() => {
    if (diffEditorRef.current) {
      const originalEditor = diffEditorRef.current.getOriginalEditor();
      const modifiedEditor = diffEditorRef.current.getModifiedEditor();

      originalEditor.updateOptions({
        lineNumbers: (lineNumber: number) => {
          const actualLine = originalLineMap[lineNumber - 1];
          return actualLine !== undefined ? String(actualLine) : String(lineNumber);
        },
      });

      modifiedEditor.updateOptions({
        lineNumbers: (lineNumber: number) => {
          const actualLine = modifiedLineMap[lineNumber - 1];
          return actualLine !== undefined ? String(actualLine) : String(lineNumber);
        },
      });
    }
  }, [originalLineMap, modifiedLineMap]);

  const handleEditorDidMount = (editor: unknown, monaco: typeof import('monaco-editor')) => {
    const diffEditor = editor as import('monaco-editor').editor.IDiffEditor;
    diffEditorRef.current = diffEditor;
    const originalEditor = diffEditor.getOriginalEditor();
    const modifiedEditor = diffEditor.getModifiedEditor();

    // Initial configuration of line numbers
    originalEditor.updateOptions({
      lineNumbers: (lineNumber: number) => {
        const actualLine = originalLineMap[lineNumber - 1];
        return actualLine !== undefined ? String(actualLine) : String(lineNumber);
      },
    });

    modifiedEditor.updateOptions({
      lineNumbers: (lineNumber: number) => {
        const actualLine = modifiedLineMap[lineNumber - 1];
        return actualLine !== undefined ? String(actualLine) : String(lineNumber);
      },
    });

    if (focusLocation) {
      const lineMap = focusLocation.side === 'old' ? originalLineMap : modifiedLineMap;
      const index = lineMap.indexOf(focusLocation.line);
      if (index >= 0) {
        const targetEditor = focusLocation.side === 'old' ? originalEditor : modifiedEditor;
        const line = index + 1;
        targetEditor.createDecorationsCollection([
          {
            range: new monaco.Range(line, 1, line, 1),
            options: { isWholeLine: true, className: 'evidence-code-highlight' },
          },
        ]);
        targetEditor.setPosition({ lineNumber: line, column: 1 });
        targetEditor.revealLineInCenter(line);
        targetEditor.focus();
      }
    }

    [originalEditor, modifiedEditor].forEach(ed => {
      // Decoration tracking per editor to prevent stuck icons
      let hoverDecorations: string[] = [];

      // Show + icon on hover anywhere in the line or gutter
      ed.onMouseMove(e => {
        // Lock the button if menu is open
        if (menuStateRef.current) return;

        const line = e.target.position?.lineNumber;
        if (line) {
          hoverDecorations = ed.deltaDecorations(hoverDecorations, [
            {
              range: new monaco.Range(line, 1, line, 1),
              options: {
                isWholeLine: true,
                glyphMarginClassName: 'add-feedback-gutter-icon',
                stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
              },
            },
          ]);
        } else {
          hoverDecorations = ed.deltaDecorations(hoverDecorations, []);
        }
      });

      // Handle click on + icon (now hamburger menu)
      ed.onMouseDown(e => {
        if (e.target.type === monaco.editor.MouseTargetType.GUTTER_GLYPH_MARGIN) {
          const line = e.target.position?.lineNumber;
          if (line) {
            // Prevent default editor behavior like selection changes if needed
            e.event.preventDefault();

            // Calculate actual line number
            const isOriginal = ed === originalEditor;
            const lineMap = isOriginal ? originalLineMap : modifiedLineMap;
            const actualLine = lineMap[line - 1];

            if (actualLine !== undefined) {
              setMenuState({
                x: e.event.posx,
                y: e.event.posy,
                file,
                line: actualLine,
                side: isOriginal ? 'old' : 'new',
              });
            }
          }
        }
      });

      // Close menu on scroll to prevent detachment
      ed.onDidScrollChange(() => {
        if (menuStateRef.current) {
          setMenuState(null);
        }
      });
    });
  };

  // Check if file is "new" (added)
  const isNewFile =
    file.status === 'added' ||
    (file.hunks.every(h => h.old_lines === 0) && file.hunks.some(h => h.new_lines > 0));

  return (
    <div className="bg-bg-primary flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      {focusLocation &&
        !(focusLocation.side === 'old' ? originalLineMap : modifiedLineMap).includes(
          focusLocation.line
        ) && (
          <p role="status" className="border-border text-text-tertiary border-b px-4 py-2 text-xs">
            Line {focusLocation.line} is outside this diff excerpt.
          </p>
        )}
      <div className="border-border flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3">
        <div className="flex items-center gap-3">
          <span className="text-text-primary font-mono text-xs break-all">{path}</span>
          {isNewFile && (
            <span className="text-status-added border-status-added/20 bg-status-added/10 rounded border px-1.5 text-[0.625rem] font-medium">
              NEW
            </span>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-1" role="group" aria-label="Diff layout">
            {(['unified', 'split'] as const).map(mode => (
              <button
                key={mode}
                aria-pressed={displayMode === mode}
                onClick={() => setDisplayMode(mode)}
                className={`rounded px-2 py-1.5 text-xs ${displayMode === mode ? 'bg-bg-tertiary text-text-primary' : 'text-text-tertiary hover:text-text-primary'}`}
              >
                {mode === 'unified' ? 'Unified' : 'Split'}
              </button>
            ))}
          </div>
          {repoRoot && (
            <button
              onClick={handleOpenInEditor}
              className="bg-bg-tertiary hover:bg-bg-secondary text-text-secondary hover:text-text-primary border-border flex items-center gap-1.5 rounded border px-2 py-1 transition-colors"
              title="Open in External Editor"
            >
              <ArrowSquareOut size={12} />
              <span className="text-xs font-medium">Open in editor</span>
            </button>
          )}
        </div>
      </div>
      {editorError && <InlineError title="Could not open editor" message={editorError} />}
      <div className="min-h-0 flex-1 overflow-hidden">
        <DiffEditor
          key={path}
          height="100%"
          language={language}
          theme="lareview-dark"
          original={original}
          modified={modified}
          onMount={handleEditorDidMount}
          originalLanguage={language}
          modifiedLanguage={language}
          options={{
            ...(focusLocation
              ? { selectionHighlight: false, occurrencesHighlight: 'off' as const }
              : {}),
            readOnly: true,
            minimap: {
              enabled: false,
              renderCharacters: false,
              scale: 2,
              showSlider: 'always',
            },
            fontSize: codeFontSize,
            lineHeight: Math.round(codeFontSize * (5 / 3)),
            fontFamily: "'GeistMono', 'Monaco', monospace",
            scrollBeyondLastLine: false,
            padding: { top: 16, bottom: 16 },
            renderSideBySide: displayMode === 'split',
            automaticLayout: true,
            originalEditable: false,
            renderLineHighlight: 'none',
            diffWordWrap: 'off' as const,
            ignoreTrimWhitespace: false,
            codeLens: false,
            folding: true,
            glyphMargin: true,
            // lineNumbers handled by useEffect/onMount to allow custom function

            renderIndicators: true,

            // Premium DX/UX enhancements
            diffAlgorithm: 'advanced',
            experimental: {
              showMoves: true,
            },
            hideUnchangedRegions: {
              enabled: !focusLocation,
              contextLineCount: 3,

              minimumLineCount: 15, // Slightly lowered for better hiding
              revealLineCount: 5,
            },
            stickyScroll: {
              enabled: true,
              maxLineCount: 5,
            },
            bracketPairColorization: {
              enabled: true,
            },
            renderWhitespace: 'boundary',
            smoothScrolling: true,
            cursorSmoothCaretAnimation: 'on',
            cursorBlinking: 'smooth',
          }}
        />
      </div>
      <p className="border-border text-text-tertiary border-t px-4 py-2 text-[0.6875rem]">
        Diff excerpt · Line numbers refer to the original files.
      </p>
      <GutterMenu
        position={menuState ? { x: menuState.x, y: menuState.y } : null}
        onClose={closeMenu}
        onAddFeedback={handleMenuAddFeedback}
        onOpenInEditor={repoRoot ? handleMenuOpenInEditor : undefined}
      />
    </div>
  );
};
