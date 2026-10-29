import { describe, expect, it } from 'vitest';
import { formatTokens, formatUsageCost, usageFromEvents } from '../review-usage';
import type { ReviewRunEvent, ReviewUsage } from '../../types';

const snapshot = (id: number, data: ReviewUsage, runId = 'run-1'): ReviewRunEvent => ({
  id,
  run_id: runId,
  review_id: 'review-1',
  created_at: '2026-10-29T10:00:00Z',
  payload: { event: 'UsageUpdated', data },
});

describe('usageFromEvents', () => {
  it('merges cumulative snapshots in persistence order without adding or losing fields', () => {
    const events = [
      snapshot(3, {
        total_tokens: 100,
        input_tokens: 60,
        output_tokens: 10,
        cached_read_tokens: 30,
      }),
      snapshot(1, {
        model: 'Effective model',
        context_used: 80,
        context_size: 100,
        cost: { amount: 0.2, currency: 'USD' },
      }),
      snapshot(2, { context_used: 20, context_size: 100, cost: { amount: 0.3, currency: 'USD' } }),
      snapshot(4, { context_used: 15, context_size: 100 }),
    ];
    expect(usageFromEvents(events, 'run-1')).toEqual({
      model: 'Effective model',
      total_tokens: 100,
      input_tokens: 60,
      output_tokens: 10,
      cached_read_tokens: 30,
      context_used: 15,
      context_size: 100,
      cost: { amount: 0.3, currency: 'USD' },
    });
    expect(events[0].id).toBe(3);
  });

  it('isolates concurrent runs and preserves zero while leaving unknown values absent', () => {
    const usage = usageFromEvents(
      [
        snapshot(1, { input_tokens: 500, cost: { amount: 20, currency: 'EUR' } }, 'other-run'),
        snapshot(2, { input_tokens: 0, cost: { amount: 0, currency: 'usd' } }),
      ],
      'run-1'
    );
    expect(usage).toEqual({ input_tokens: 0, cost: { amount: 0, currency: 'USD' } });
    expect(usage.output_tokens).toBeUndefined();
  });

  it('ignores malformed stored counts and currencies without destroying valid data', () => {
    expect(
      usageFromEvents(
        [
          snapshot(1, { total_tokens: 100, cost: { amount: 1, currency: 'EUR' } }),
          snapshot(2, {
            total_tokens: -1,
            input_tokens: NaN,
            output_tokens: Infinity,
            cost: { amount: -1, currency: 'USD' },
          }),
          snapshot(3, { cost: { amount: 1, currency: '?' } }),
        ],
        'run-1'
      )
    ).toEqual({ total_tokens: 100, cost: { amount: 1, currency: 'EUR' } });
    expect(usageFromEvents([], 'run-1')).toEqual({});
  });
});

describe('usage formatting', () => {
  it('distinguishes free, unknown, and very small costs', () => {
    expect(formatUsageCost(undefined)).toBe('Not reported');
    expect(formatUsageCost({ amount: 0, currency: 'USD' })).toBe('$0.00');
    expect(formatUsageCost({ amount: 0.0023, currency: 'USD' })).toBe('$0.0023');
    expect(formatUsageCost({ amount: 0.000001, currency: 'USD' })).toBe('< $0.0001');
    expect(formatTokens(undefined)).toBe('—');
    expect(formatTokens(0)).toBe('0');
    expect(formatTokens(128400)).toBe('128.4K');
  });
});
