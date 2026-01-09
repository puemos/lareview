import React, { useState, useEffect, useRef, useCallback } from 'react';
import ReactMarkdown from 'react-markdown';
import { useVirtualizer } from '@tanstack/react-virtual';
import { CaretRight, CaretDown, TerminalWindow, Warning, Code, Check } from '@phosphor-icons/react';
import clsx from 'clsx';

interface TimelineProps {
  messages: Array<{
    type: string;
    message: string;
    data?: Record<string, unknown>;
    timestamp: number;
  }>;
}

export const Timeline: React.FC<TimelineProps> = ({ messages }) => {
  const parentRef = useRef<HTMLDivElement>(null);
  const [isAtBottom, setIsAtBottom] = useState(true);
  const rafIdRef = useRef<number | null>(null);

  // Filter out system/debug logs for cleaner UI
  const visibleMessages = messages.filter(
    m => !['system', 'log', 'debug', 'task_started', 'task_added'].includes(m.type)
  );

  // useVirtualizer is incompatible with React 19's strict purity rules
  // eslint-disable-next-line react-hooks/incompatible-library
  const rowVirtualizer = useVirtualizer({
    count: visibleMessages.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 40, // Reduced estimate for collapsed items
    overscan: 20,
    measureElement: element => {
      return element.getBoundingClientRect().height;
    },
  });

  // Auto-scroll logic
  useEffect(() => {
    if (visibleMessages.length === 0) return;

    if (isAtBottom) {
      if (rafIdRef.current) {
        clearTimeout(rafIdRef.current);
      }
      rafIdRef.current = window.setTimeout(() => {
        if (rowVirtualizer) {
          try {
            rowVirtualizer.scrollToIndex(visibleMessages.length - 1, { align: 'end' });
          } catch {
            // ignore scroll errors
          }
        }
        rafIdRef.current = null;
      }, 50);
    }
  }, [visibleMessages.length, isAtBottom, rowVirtualizer]);

  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const element = e.currentTarget;
    // Use a slightly larger tolerance to make "sticking" easier
    const atBottom = Math.abs(element.scrollHeight - element.scrollTop - element.clientHeight) < 100;
    setIsAtBottom(atBottom);
  }, []);

  if (visibleMessages.length === 0) {
    return (
      <div className="text-text-disabled flex h-full flex-col items-center justify-center space-y-2 font-mono opacity-50">
        <TerminalWindow size={32} />
        <p className="text-xs">{'>'} ready_to_generate...</p>
      </div>
    );
  }

  const virtualItems = rowVirtualizer.getVirtualItems();

  return (
    <div className="relative flex h-full flex-col">
      <div
        className="flex-1 overflow-y-auto scroll-smooth"
        ref={parentRef}
        onScroll={handleScroll}
        style={{ scrollBehavior: 'smooth' }}
      >
        <div
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: '100%',
            position: 'relative',
          }}
        >
          {virtualItems.map(virtualRow => {
            const msg = visibleMessages[virtualRow.index];
            return (
              <div
                key={virtualRow.key}
                data-index={virtualRow.index}
                ref={rowVirtualizer.measureElement}
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  transform: `translateY(${virtualRow.start}px)`,
                  padding: '0.25rem 1rem', // Reduced padding
                }}
              >
                {renderMessage(msg, rowVirtualizer)}
              </div>
            );
          })}
        </div>
      </div>

      {!isAtBottom && visibleMessages.length > 0 && (
        <button
          onClick={() => {
            rowVirtualizer.scrollToIndex(visibleMessages.length - 1, { align: 'end' });
            setIsAtBottom(true);
          }}
          className="bg-bg-tertiary border-border text-text-secondary hover:text-text-primary animate-fade-in absolute bottom-6 left-1/2 z-10 flex -translate-x-1/2 transform items-center gap-2 rounded-full border px-3 py-1.5 font-mono text-[10px] shadow-xl transition-all"
        >
          <span>↓ Jump to Bottom</span>
        </button>
      )}
    </div>
  );
};

function renderMessage(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  msg: { type: string; message: string; data?: any; timestamp: number },
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  virtualizer: any
) {
  switch (msg.type) {
    case 'agent_thought':
      return <ThinkingItem text={msg.message} virtualizer={virtualizer} />;
    case 'agent_message':
      return <MessageItem text={msg.message} />;
    case 'tool_call':
      return <ToolCallItem data={msg.data} virtualizer={virtualizer} />;
    case 'error':
      return (
        <div className="text-status-deleted animate-fade-in my-1 flex gap-2 px-0 py-2 font-mono text-xs">
          <div className="shrink-0">
            <Warning size={14} weight="fill" />
          </div>
          <div className="break-words whitespace-pre-wrap">
            {'>'} Error: {msg.message}
          </div>
        </div>
      );
    case 'completed':
      return (
        <div className="text-success animate-fade-in border-border/30 my-1 mt-4 flex justify-center gap-2 border-t border-dashed px-0 py-4 font-mono text-xs">
          <div className="shrink-0">
            <Check size={14} weight="bold" />
          </div>
          <div>{'>'} REVIEW_GENERATION_COMPLETE</div>
        </div>
      );
    default:
      return null;
  }
}

// -- Subcomponents --

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const ThinkingItem = ({ text, virtualizer }: { text: string; virtualizer: any }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);

  // Clean up thinking text
  const cleanText = text.replace(/<thinking>|<\/thinking>/g, '').trim();

  const toggle = () => {
    setIsExpanded(!isExpanded);
    // Give time for expansion then remeasure
    setTimeout(() => {
      if (contentRef.current?.parentElement?.parentElement) {
        virtualizer.measureElement(contentRef.current.parentElement.parentElement);
      }
    }, 50);
  };

  if (!cleanText) return null;

  return (
    <div ref={contentRef} className="animate-fade-in group flex flex-col gap-1 py-1">
      <button
        onClick={toggle}
        className="text-text-tertiary hover:text-text-secondary flex w-fit items-center gap-2 font-mono text-[10px] transition-colors select-none"
      >
        {isExpanded ? <CaretDown size={10} /> : <CaretRight size={10} />}
        <span>{'>'} thinking_process...</span>
      </button>

      {isExpanded && (
        <div className="border-border/30 my-1 ml-1 border-l pl-4">
          <div className="markdown-content text-text-secondary font-mono text-xs leading-relaxed opacity-80">
            <ReactMarkdown>{cleanText}</ReactMarkdown>
          </div>
        </div>
      )}
    </div>
  );
};

const MessageItem = ({ text }: { text: string }) => {
  return (
    <div className="animate-fade-in group flex gap-3 py-2">
      <div className="min-w-0 flex-1 space-y-1">
        <div className="markdown-content text-text-primary font-sans text-sm leading-relaxed break-words whitespace-pre-wrap">
          <ReactMarkdown>{text}</ReactMarkdown>
        </div>
      </div>
    </div>
  );
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const ToolCallItem = ({ data, virtualizer }: { data: any; virtualizer: any }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);

  // Safely extract content
  const dataWithStrings = data as Record<
    string,
    string | undefined | Record<string, unknown> | unknown
  >;
  let displayTitle = (dataWithStrings.title as string) || 'Using Tool...';
  const rawInput =
    (dataWithStrings.rawInput as string) ||
    ((dataWithStrings.fields as Record<string, unknown>)?.rawInput as string);
  const rawOutput = (dataWithStrings.rawOutput as string) || (dataWithStrings.raw_output as string);
  const status = ((dataWithStrings.status as string) ||
    ((dataWithStrings.fields as Record<string, unknown>)?.status as string) ||
    'InProgress') as string;
  const isFailed = status === 'Failed';
  const isCompleted = status === 'Completed';

  // Parse title
  if (displayTitle.includes('{')) {
    try {
      const idx = displayTitle.indexOf('{');
      const prefix = displayTitle.substring(0, idx).trim();
      if (prefix) displayTitle = prefix;
    } catch {
      // ignore parse errors
    }
  }
  displayTitle = displayTitle
    .replace(/^call_\w+/, '')
    .replace(/["']/g, '')
    .trim();

  const toggle = () => {
    setIsExpanded(!isExpanded);
    setTimeout(() => {
      if (contentRef.current?.parentElement?.parentElement) {
        virtualizer.measureElement(contentRef.current.parentElement.parentElement);
      }
    }, 50);
  };

  return (
    <div ref={contentRef} className="animate-fade-in group my-1 py-0.5">
      <div
        onClick={toggle}
        className="-mx-1.5 flex cursor-pointer items-center gap-2 rounded px-1.5 py-1 transition-colors select-none hover:bg-white/5"
      >
        <div className="text-text-tertiary group-hover:text-text-primary shrink-0 transition-colors">
          {isExpanded ? <CaretDown size={12} /> : <CaretRight size={12} />}
        </div>

        <div className="flex min-w-0 flex-1 items-center gap-2">
          <Code size={12} className="text-brand/70 shrink-0" />
          <span className="text-text-secondary group-hover:text-text-primary truncate font-mono text-xs transition-colors">
            {displayTitle}
          </span>
        </div>

        <div
          className={clsx(
            'shrink-0 font-mono text-[10px]',
            isFailed
              ? 'text-status-deleted'
              : isCompleted
                ? 'text-success'
                : 'text-brand animate-pulse'
          )}
        >
          [{isFailed ? 'FAILED' : isCompleted ? 'DONE' : 'RUNNING'}]
        </div>
      </div>

      {isExpanded && (
        <div className="border-border/30 mt-1 ml-[5px] overflow-hidden border-l pl-4">
          <div className="space-y-2 py-1">
            <div className="space-y-0.5">
              <div className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
                Input
              </div>
              <div className="text-text-secondary bg-bg-tertiary/40 border-border/50 overflow-x-auto rounded border p-2 font-mono text-[10px]">
                <pre className="break-all whitespace-pre-wrap">
                  {typeof rawInput === 'object'
                    ? JSON.stringify(rawInput, null, 2)
                    : rawInput || 'No input'}
                </pre>
              </div>
            </div>

            {rawOutput && (
              <div className="space-y-0.5">
                <div className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
                  Output
                </div>
                <div className="text-text-secondary bg-bg-tertiary/40 border-border/50 max-h-[200px] overflow-x-auto rounded border p-2 font-mono text-[10px]">
                  <pre className="break-all whitespace-pre-wrap">
                    {typeof rawOutput === 'object' ? JSON.stringify(rawOutput, null, 2) : rawOutput}
                  </pre>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
