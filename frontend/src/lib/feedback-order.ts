import type { Feedback } from '../types';
export function sortFeedback(feedbacks: Feedback[]): Feedback[] {
  const statusRank = (status: Feedback['status']) =>
    status === 'done' ? 1 : status === 'ignored' ? 2 : 0;
  const impactRank = { blocking: 0, nice_to_have: 1, nitpick: 2 };
  const timestamp = (value: string) => {
    const time = Date.parse(value);
    return Number.isFinite(time) ? time : 0;
  };
  return [...feedbacks].sort(
    (a, b) =>
      statusRank(a.status) - statusRank(b.status) ||
      impactRank[a.impact] - impactRank[b.impact] ||
      timestamp(b.updated_at) - timestamp(a.updated_at)
  );
}
