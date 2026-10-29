import type { Feedback, IssueCheckWithFindings } from '../types';

/** Linked feedback is the current disposition of a finding, including resolution/impact edits. */
export function countOpenBlockers(feedbacks: Feedback[], checks: IssueCheckWithFindings[]): number {
  const linkedFindings = new Set(
    feedbacks.flatMap(feedback => (feedback.finding_id ? [feedback.finding_id] : []))
  );
  const blockers = new Set(
    feedbacks
      .filter(
        feedback =>
          feedback.impact === 'blocking' &&
          feedback.status !== 'done' &&
          feedback.status !== 'ignored'
      )
      .map(feedback =>
        feedback.finding_id ? `finding:${feedback.finding_id}` : `feedback:${feedback.id}`
      )
  );
  for (const check of checks) {
    for (const finding of check.findings) {
      if (finding.impact === 'blocking' && !linkedFindings.has(finding.id))
        blockers.add(`finding:${finding.id}`);
    }
  }
  return blockers.size;
}
