import { describe, expect, it } from 'vitest';
import type { Feedback, IssueCheckWithFindings } from '../types';
import { countOpenBlockers } from './review-summary';
describe('countOpenBlockers', () => {
  const checks = [
    { findings: [{ id: 'finding', impact: 'blocking' }] },
  ] as IssueCheckWithFindings[];
  it('counts a linked finding once and honors its current disposition', () => {
    const feedback = {
      id: 'feedback',
      finding_id: 'finding',
      impact: 'blocking',
      status: 'todo',
    } as Feedback;
    expect(countOpenBlockers([feedback], checks)).toBe(1);
    expect(countOpenBlockers([{ ...feedback, status: 'done' }], checks)).toBe(0);
    expect(countOpenBlockers([{ ...feedback, status: 'ignored' }], checks)).toBe(0);
    expect(countOpenBlockers([{ ...feedback, impact: 'nitpick' }], checks)).toBe(0);
  });
  it('includes unlinked findings and independent blocking feedback', () => {
    expect(
      countOpenBlockers(
        [{ id: 'other', impact: 'blocking', status: 'in_progress' } as Feedback],
        checks
      )
    ).toBe(2);
  });
});
