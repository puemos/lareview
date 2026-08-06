import React, { useCallback, useLayoutEffect, useRef, useState } from 'react';
import { TerminalWindow } from '@phosphor-icons/react';
import clsx from 'clsx';

export interface ProgressMessage {
  type: string;
  message: string;
  data?: Record<string, unknown>;
  timestamp: number;
  id?: string | number;
}

interface LiveActivityFeedProps {
  messages: ProgressMessage[];
  isRunning: boolean;
}

export const LiveActivityFeed: React.FC<LiveActivityFeedProps> = ({ messages, isRunning }) => {
  const scrollRef = useRef<HTMLDivElement>(null);
  const shouldFollowRef = useRef(true);
  const [isFollowing, setIsFollowing] = useState(true);

  const updateFollowState = useCallback(() => {
    const element = scrollRef.current;
    if (!element) return;
    const distanceFromBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
    const nextIsFollowing = distanceFromBottom <= 32;
    shouldFollowRef.current = nextIsFollowing;
    setIsFollowing(nextIsFollowing);
  }, []);

  const jumpToLatest = useCallback(() => {
    const element = scrollRef.current;
    if (!element) return;
    shouldFollowRef.current = true;
    setIsFollowing(true);
    element.scrollTop = element.scrollHeight;
  }, []);

  useLayoutEffect(() => {
    if (!shouldFollowRef.current) return;
    const frame = requestAnimationFrame(() => {
      const element = scrollRef.current;
      if (element) element.scrollTop = element.scrollHeight;
    });
    return () => cancelAnimationFrame(frame);
  }, [messages]);

  return (
    <div className="bg-bg-primary/50 relative flex min-h-[150px] flex-1 flex-col">
      <div className="border-border bg-bg-secondary flex items-center justify-between border-b px-4 py-2">
        <h2 className="text-text-disabled flex items-center gap-2 text-[10px] font-bold tracking-wider uppercase">
          <TerminalWindow size={14} />
          Activity
        </h2>
        {isRunning && (
          <span className="flex items-center gap-1.5">
            <span className="bg-success h-1.5 w-1.5 animate-pulse rounded-full" />
            <span className="text-success font-mono text-[10px]">Running</span>
          </span>
        )}
      </div>
      <div
        ref={scrollRef}
        onScroll={updateFollowState}
        data-testid="activity-scroll"
        className="custom-scrollbar flex-1 overflow-y-auto"
      >
        {messages.length === 0 ? (
          <div className="text-text-disabled p-8 text-center text-xs opacity-50">
            No activity yet
          </div>
        ) : (
          <Timeline messages={messages} />
        )}
      </div>
      {!isFollowing && messages.length > 0 && (
        <button
          type="button"
          onClick={jumpToLatest}
          className="bg-bg-tertiary border-border text-text-secondary hover:text-text-primary shadow-custom absolute right-4 bottom-4 rounded-md border px-2.5 py-1.5 text-[10px] font-medium transition-colors active:scale-[0.97]"
        >
          Jump to latest
        </button>
      )}
    </div>
  );
};

interface TimelineProps {
  messages: ProgressMessage[];
}

const Timeline: React.FC<TimelineProps> = ({ messages }) => {
  return (
    <div className="space-y-1 p-3">
      {messages.map((msg, index) => (
        <TimelineItem key={msg.id ?? `${msg.timestamp}-${msg.type}-${index}`} message={msg} />
      ))}
    </div>
  );
};

const TimelineItem: React.FC<{ message: ProgressMessage }> = ({ message }) => {
  const getTypeStyles = () => {
    switch (message.type) {
      case 'agent_message':
        return 'text-blue-400 bg-blue-400/10';
      case 'agent_thought':
        return 'text-purple-400 bg-purple-400/10';
      case 'tool_call':
        return 'text-yellow-400 bg-yellow-400/10';
      case 'error':
        return 'text-red-400 bg-red-400/10';
      case 'completed':
        return 'text-green-400 bg-green-400/10';
      case 'log':
        return 'text-gray-400 bg-gray-400/10';
      default:
        return 'text-text-secondary bg-bg-tertiary';
    }
  };

  return (
    <div className="animate-fade-in group flex flex-col gap-1.5 rounded-md p-2 transition-colors hover:bg-white/5">
      {/* Header row: Tag and Timestamp */}
      <div className="flex items-center justify-between border-b border-white/5 pb-1">
        <span
          className={clsx(
            'rounded px-1.5 py-0.5 text-[9px] font-bold tracking-wider uppercase',
            getTypeStyles()
          )}
        >
          {message.type.replace(/_/g, ' ')}
        </span>
        <span className="text-text-disabled font-mono text-[9px] opacity-50 transition-opacity group-hover:opacity-80">
          {formatTimestamp(message.timestamp)}
        </span>
      </div>

      {/* Content row */}
      <div className="text-text-secondary pl-1 text-xs leading-relaxed break-words whitespace-pre-wrap">
        {message.message}
      </div>
    </div>
  );
};

const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
};
