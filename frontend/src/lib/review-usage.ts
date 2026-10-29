import type { ReviewRunEvent, ReviewUsage } from '../types';

const tokenFields = [
  'total_tokens',
  'input_tokens',
  'output_tokens',
  'cached_read_tokens',
  'cached_write_tokens',
  'thought_tokens',
  'context_used',
  'context_size',
] as const;

/** Merge snapshots in persistence order. Context can shrink after compaction; costs are not deltas. */
export function usageFromEvents(events: ReviewRunEvent[], runId: string): ReviewUsage {
  const usage: ReviewUsage = {};
  for (const event of [...events].sort((a, b) => a.id - b.id)) {
    if (event.run_id !== runId || event.payload.event !== 'UsageUpdated') continue;
    const data = event.payload.data as ReviewUsage;
    if (!data || typeof data !== 'object') continue;
    if (typeof data.model === 'string' && data.model.trim()) usage.model = data.model;
    for (const field of tokenFields) {
      const value = data[field];
      if (typeof value === 'number' && Number.isSafeInteger(value) && value >= 0) {
        usage[field] = value;
      }
    }
    if (
      data.cost &&
      Number.isFinite(data.cost.amount) &&
      data.cost.amount >= 0 &&
      /^[a-z]{3}$/i.test(data.cost.currency)
    ) {
      usage.cost = { ...data.cost, currency: data.cost.currency.toUpperCase() };
    }
  }
  return usage;
}

export function formatTokens(value?: number): string {
  if (value === undefined) return '—';
  return new Intl.NumberFormat('en-US', {
    notation: value >= 10_000 ? 'compact' : 'standard',
    maximumFractionDigits: value >= 10_000 ? 1 : 0,
  }).format(value);
}

export function formatUsageCost(cost: ReviewUsage['cost']): string {
  if (!cost) return 'Not reported';
  const formatted = new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: cost.currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: cost.amount > 0 && cost.amount < 0.01 ? 4 : 2,
  });
  if (cost.amount > 0 && cost.amount < 0.0001) return `< ${formatted.format(0.0001)}`;
  return formatted.format(cost.amount);
}
