import { expect, it } from 'vitest';
import type { Feedback } from '../types';
import { sortFeedback } from './feedback-order';
it('prioritizes active blockers and sorts resolved feedback last without mutating input', () => {
  const items = [
    { id: 'done', status: 'done', impact: 'blocking' },
    { id: 'nice', status: 'todo', impact: 'nice_to_have' },
    { id: 'block', status: 'in_progress', impact: 'blocking' },
  ] as Feedback[];
  expect(sortFeedback(items).map(item => item.id)).toEqual(['block', 'nice', 'done']);
  expect(items[0].id).toBe('done');
});
