import React, { useMemo, useState } from 'react';
import { SpinnerGap, Stop } from '@phosphor-icons/react';
import { useGeneration } from '../../contexts/useGeneration';
import { useReviewRunEvents } from '../../hooks/useReviewRunEvents';
import type { Plan } from '../../types';
import type { ProgressEventPayload } from '../../hooks/useTauri';
import { LiveActivityFeed, type ProgressMessage } from '../Generate/LiveActivityFeed';
import { PlanOverview } from '../Generate/PlanOverview';
import type { Review, ReviewRun, ReviewRunEvent } from '../../types';

interface ReviewGenerationActivityProps {
  review: Review | null;
  run: ReviewRun;
}

const statusCopy: Record<ReviewRun['status'], string> = {
  queued: 'Queued',
  running: 'Analyzing',
  completed: 'Ready',
  failed: 'Failed',
  cancelled: 'Cancelled',
  interrupted: 'Interrupted',
};

function eventTimestamp(event: ReviewRunEvent): number {
  const timestamp = Date.parse(event.created_at);
  return Number.isNaN(timestamp) ? Date.now() : timestamp;
}

function activityFromEvents(events: ReviewRunEvent[]) {
  let plan: Plan | null = null;
  const messages: ProgressMessage[] = [];
  const messageIndexes = new Map<string, number>();
  const thoughtIndexes = new Map<string, number>();
  const toolIndexes = new Map<string, number>();

  const appendDelta = (
    event: ReviewRunEvent,
    data: { id: string; delta: string },
    type: 'agent_message' | 'agent_thought',
    indexes: Map<string, number>
  ) => {
    const existingIndex = indexes.get(data.id);
    if (existingIndex === undefined) {
      indexes.set(data.id, messages.length);
      messages.push({
        id: `${type}-${data.id}`,
        type,
        message: data.delta,
        timestamp: eventTimestamp(event),
      });
      return;
    }
    messages[existingIndex] = {
      ...messages[existingIndex],
      message: messages[existingIndex].message + data.delta,
      timestamp: eventTimestamp(event),
    };
  };

  for (const event of events) {
    const payload = event.payload as ProgressEventPayload;
    switch (payload.event) {
      case 'Plan':
        plan = payload.data as Plan;
        break;
      case 'MessageDelta':
        appendDelta(
          event,
          payload.data as { id: string; delta: string },
          'agent_message',
          messageIndexes
        );
        break;
      case 'ThoughtDelta':
        appendDelta(
          event,
          payload.data as { id: string; delta: string },
          'agent_thought',
          thoughtIndexes
        );
        break;
      case 'ToolCallStarted': {
        const data = payload.data as { tool_call_id: string; title: string; kind: string };
        toolIndexes.set(data.tool_call_id, messages.length);
        messages.push({
          id: `tool-${data.tool_call_id}`,
          type: 'tool_call',
          message: data.title,
          data: { status: 'running', kind: data.kind },
          timestamp: eventTimestamp(event),
        });
        break;
      }
      case 'ToolCallComplete': {
        const data = payload.data as {
          tool_call_id: string;
          title: string;
          status: string;
          raw_input?: unknown;
          raw_output?: unknown;
        };
        const existingIndex = toolIndexes.get(data.tool_call_id);
        const next: ProgressMessage = {
          id: `tool-${data.tool_call_id}`,
          type: 'tool_call',
          message: data.title,
          data: {
            status: data.status,
            raw_input: data.raw_input,
            raw_output: data.raw_output,
          },
          timestamp: eventTimestamp(event),
        };
        if (existingIndex === undefined) {
          toolIndexes.set(data.tool_call_id, messages.length);
          messages.push(next);
        } else {
          messages[existingIndex] = next;
        }
        break;
      }
      case 'Status': {
        const data = payload.data as { status: string };
        messages.push({
          id: event.id,
          type: 'log',
          message: data.status === 'running' ? 'Analysis started' : `Review ${data.status}`,
          timestamp: eventTimestamp(event),
        });
        break;
      }
      case 'Log':
        messages.push({
          id: event.id,
          type: 'log',
          message: payload.data as string,
          timestamp: eventTimestamp(event),
        });
        break;
      case 'TaskStarted': {
        const data = payload.data as { title: string };
        messages.push({
          id: event.id,
          type: 'task_started',
          message: data.title,
          timestamp: eventTimestamp(event),
        });
        break;
      }
      case 'TaskCompleted':
        messages.push({
          id: event.id,
          type: 'task_added',
          message: 'Review task added',
          timestamp: eventTimestamp(event),
        });
        break;
      case 'Completed':
        messages.push({
          id: event.id,
          type: 'completed',
          message: 'Review generation complete',
          timestamp: eventTimestamp(event),
        });
        break;
      case 'Error': {
        const data = payload.data as { message: string };
        messages.push({
          id: event.id,
          type: 'error',
          message: data.message,
          timestamp: eventTimestamp(event),
        });
        break;
      }
    }
  }

  return { plan, messages };
}

export const ReviewGenerationActivity: React.FC<ReviewGenerationActivityProps> = ({
  review,
  run,
}) => {
  const { stopGeneration } = useGeneration();
  const { data: events = [] } = useReviewRunEvents(run.id);
  const [isStopping, setIsStopping] = useState(false);
  const { plan, messages } = useMemo(() => activityFromEvents(events), [events]);
  const isActive = run.status === 'queued' || run.status === 'running';

  const handleStop = async () => {
    setIsStopping(true);
    await stopGeneration(run.id);
  };

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <div className="border-border bg-bg-secondary/50 flex items-center justify-between border-b px-6 py-4">
        <div className="min-w-0">
          <div className="text-text-disabled mb-1 flex items-center gap-2 text-[10px] font-bold tracking-wider uppercase">
            {isActive && <SpinnerGap size={12} className="text-brand animate-spin" />}
            <span>{statusCopy[run.status]}</span>
            <span className="text-border">·</span>
            <span>{run.agent_id}</span>
          </div>
          <h2 className="text-text-primary truncate text-lg font-semibold">
            {review?.title || 'New Review'}
          </h2>
          <p className="text-text-tertiary mt-0.5 text-xs">
            {run.status === 'queued'
              ? 'Waiting for an available review slot'
              : run.status === 'running'
                ? 'You can leave this page. The review will keep running in the background.'
                : 'Generation stopped before the review was completed.'}
          </p>
        </div>
        {isActive && (
          <button
            type="button"
            onClick={handleStop}
            disabled={isStopping}
            className="border-status-ignored/20 bg-status-ignored/10 text-status-ignored hover:bg-status-ignored/20 flex items-center gap-2 rounded-md border px-3 py-2 text-xs font-medium transition-colors active:scale-[0.97] disabled:cursor-wait disabled:opacity-60"
          >
            {isStopping ? <SpinnerGap size={14} className="animate-spin" /> : <Stop size={14} />}
            {isStopping ? 'Stopping…' : 'Stop'}
          </button>
        )}
      </div>

      <div className="flex min-h-0 flex-1 overflow-hidden">
        <div className="border-border flex min-w-0 flex-1 border-r">
          <LiveActivityFeed messages={messages} isRunning={isActive} />
        </div>
        <div className="bg-bg-secondary flex w-[380px] flex-col">
          <PlanOverview
            items={
              plan?.entries.map(entry => ({
                content: entry.content,
                status: entry.status || 'pending',
              })) || []
            }
            emptyMessage="The agent's plan will appear here once analysis begins."
          />
        </div>
      </div>
    </div>
  );
};
