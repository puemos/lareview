import { useId, useMemo, useRef } from 'react';
import * as Popover from '@radix-ui/react-popover';
import { ChartBar, CircleNotch, Info, X } from '@phosphor-icons/react';
import { useReviewRunEvents } from '../../hooks/useReviewRunEvents';
import { formatTokens, formatUsageCost, usageFromEvents } from '../../lib/review-usage';
import type { ReviewRun, ReviewUsage } from '../../types';
import { ReviewHeaderBadge } from './ReviewHeaderBadge';

export function ReviewUsagePopover({ run }: { run: ReviewRun }) {
  const { data: events = [], isLoading, isError, refetch } = useReviewRunEvents(run.id);
  const usage = useMemo(() => usageFromEvents(events, run.id), [events, run.id]);
  return (
    <UsagePopover
      key={run.id}
      usage={usage}
      agentId={run.agent_id}
      isActive={run.status === 'queued' || run.status === 'running'}
      isLoading={isLoading}
      isError={isError}
      onRetry={() => void refetch()}
    />
  );
}

interface UsagePopoverProps {
  usage: ReviewUsage;
  agentId: string;
  isActive?: boolean;
  isLoading?: boolean;
  isError?: boolean;
  onRetry?: () => void;
}

export function UsagePopover({
  usage,
  agentId,
  isActive = false,
  isLoading = false,
  isError = false,
  onRetry,
}: UsagePopoverProps) {
  const titleId = useId();
  const contentRef = useRef<HTMLDivElement>(null);
  const agentName = agentId.charAt(0).toUpperCase() + agentId.slice(1);
  const hasUsage = Object.keys(usage).some(key => key !== 'model');
  const contextPercent =
    usage.context_used !== undefined && usage.context_size && usage.context_size > 0
      ? Math.min(100, (usage.context_used / usage.context_size) * 100)
      : null;
  const cost = formatUsageCost(usage.cost);
  const triggerLabel = isLoading
    ? 'Loading AI usage'
    : usage.cost
      ? `Estimated cost ${cost}. View AI usage`
      : 'View AI usage';

  return (
    <Popover.Root>
      <Popover.Trigger asChild>
        <ReviewHeaderBadge
          aria-label={triggerLabel}
          aria-busy={isLoading || undefined}
          className="border-border text-text-secondary hover:bg-bg-tertiary data-[state=open]:bg-bg-tertiary bg-transparent"
          icon={
            isLoading ? <CircleNotch size={16} className="animate-spin" /> : <ChartBar size={16} />
          }
        >
          {usage.cost ? cost : 'Usage'}
        </ReviewHeaderBadge>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          ref={contentRef}
          tabIndex={-1}
          side="bottom"
          align="end"
          sideOffset={8}
          collisionPadding={12}
          aria-labelledby={titleId}
          onOpenAutoFocus={event => {
            event.preventDefault();
            contentRef.current?.focus();
          }}
          className="border-border bg-bg-elevated text-text-primary z-50 [max-height:var(--radix-popover-content-available-height)] w-80 max-w-[calc(100vw-24px)] [transform-origin:var(--radix-popover-content-transform-origin)] overflow-y-auto rounded-lg border shadow-xl outline-none"
        >
          <div className="flex items-start justify-between gap-3 px-4 pt-4 pb-3">
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <h3 id={titleId} className="text-sm font-medium">
                  AI usage
                </h3>
                {isActive && (
                  <span className="text-status-done flex items-center gap-1 text-[0.625rem]">
                    <span className="h-1 w-1 rounded-full bg-current" />
                    Live
                  </span>
                )}
              </div>
              <p className="text-text-tertiary mt-1 text-[0.6875rem] leading-relaxed [overflow-wrap:anywhere]">
                {usage.model || agentName}
                {usage.model && <span className="text-text-disabled"> · {agentName}</span>}
              </p>
            </div>
            <Popover.Close asChild>
              <button
                type="button"
                aria-label="Close usage details"
                className="text-text-disabled hover:text-text-primary hover:bg-bg-tertiary focus-visible:outline-brand -mt-1 -mr-1 flex h-7 w-7 shrink-0 items-center justify-center rounded-md transition-colors focus-visible:outline-2 focus-visible:outline-offset-2"
              >
                <X size={14} />
              </button>
            </Popover.Close>
          </div>

          {isLoading || isError || !hasUsage ? (
            <div className="flex items-start gap-2.5 px-4 pt-1 pb-4 text-xs leading-relaxed">
              <Info size={15} className="text-text-disabled mt-0.5 shrink-0" />
              <div>
                <p className="text-text-primary font-medium">
                  {isLoading
                    ? 'Loading usage…'
                    : isError
                      ? 'Usage could not be loaded'
                      : isActive
                        ? 'Waiting for usage'
                        : 'No usage reported'}
                </p>
                <p className="text-text-disabled mt-1">
                  {isLoading
                    ? 'Retrieving recorded usage.'
                    : isError
                      ? 'Try again to load saved usage.'
                      : isActive
                        ? 'Details will appear when the agent reports them.'
                        : 'No token counts or cost were recorded for this generation.'}
                </p>
                {isError && (
                  <button
                    type="button"
                    onClick={onRetry}
                    className="text-brand focus-visible:outline-brand mt-2 rounded hover:underline focus-visible:outline-2 focus-visible:outline-offset-2"
                  >
                    Try again
                  </button>
                )}
              </div>
            </div>
          ) : (
            <>
              <div className="px-4 pt-1 pb-4">
                <div className="flex items-center justify-between gap-3">
                  <span className="text-text-tertiary text-xs">Estimated cost</span>
                  <span className="text-text-tertiary text-[0.6875rem]">
                    {isActive ? 'This generation so far' : 'This generation'}
                  </span>
                </div>
                <p
                  className={`mt-2 flex items-baseline gap-1.5 leading-none font-medium tracking-tight tabular-nums ${usage.cost ? 'text-2xl' : 'text-text-disabled text-lg'}`}
                >
                  {cost}
                  {usage.cost && (
                    <span className="text-text-tertiary text-[0.625rem] font-normal tracking-normal">
                      {usage.cost.currency}
                    </span>
                  )}
                </p>
              </div>

              <dl className="border-border/50 space-y-2 border-t px-4 py-3.5 text-xs">
                <TokenRow label="Input tokens" value={usage.input_tokens} color="bg-accent" />
                <TokenRow label="Output tokens" value={usage.output_tokens} color="bg-brand" />
                <TokenRow
                  label="Cache read"
                  value={usage.cached_read_tokens}
                  color="bg-status-done"
                />
                <TokenRow label="Cache write" value={usage.cached_write_tokens} />
                <TokenRow label="Reasoning tokens" value={usage.thought_tokens} />
                <div className="border-border/50 flex items-center justify-between border-t pt-2.5">
                  <dt className="text-text-secondary">Total tokens</dt>
                  <dd className="font-mono tabular-nums">
                    {usage.total_tokens?.toLocaleString('en-US') ?? '—'}
                  </dd>
                </div>
              </dl>

              {contextPercent !== null && (
                <div className="border-border/50 border-t px-4 py-3.5">
                  <div className="text-text-tertiary mb-2 flex items-center justify-between gap-3 text-[0.6875rem]">
                    <span>Context window</span>
                    <span className="tabular-nums">
                      {formatTokens(usage.context_used)} / {formatTokens(usage.context_size)}
                      <span className="text-text-secondary"> · {Math.round(contextPercent)}%</span>
                    </span>
                  </div>
                  <div
                    role="meter"
                    aria-label="Context window used"
                    aria-valuenow={Math.round(contextPercent)}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuetext={`${Math.round(contextPercent)}% of context window`}
                    className="bg-bg-tertiary h-1 overflow-hidden rounded-full"
                  >
                    <div
                      className={`h-full rounded-full ${contextPercent >= 90 ? 'bg-impact-nice_to_have' : 'bg-brand/70'}`}
                      style={{ width: `${contextPercent}%` }}
                    />
                  </div>
                </div>
              )}

              <div className="border-border/50 bg-bg-secondary/30 text-text-tertiary border-t px-4 py-3 text-[0.6875rem] leading-relaxed">
                {usage.cost
                  ? 'Agent-reported estimate. Billing may differ by plan. A dash means not reported.'
                  : 'Cost was not reported. Token counts do not determine subscription charges. A dash means not reported.'}
              </div>
            </>
          )}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

function TokenRow({ label, value, color }: { label: string; value?: number; color?: string }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <dt className="text-text-tertiary flex items-center gap-2">
        <span className={`h-1.5 w-1.5 rounded-full ${color || 'bg-text-disabled/50'}`} />
        {label}
      </dt>
      <dd
        title={value?.toLocaleString('en-US')}
        className="text-text-secondary font-mono tabular-nums"
      >
        {value?.toLocaleString('en-US') ?? '—'}
      </dd>
    </div>
  );
}
