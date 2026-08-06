import { beforeEach, describe, expect, it } from 'vitest';
import { useAppStore } from '../index';

describe('AppStore', () => {
  beforeEach(() => {
    useAppStore.getState().reset();
  });

  it('clears review-specific selections when reset', () => {
    const store = useAppStore.getState();
    store.setReviewId('review-one');
    store.selectTask('task-one');
    store.setDiffText('diff');

    useAppStore.getState().reset();

    expect(useAppStore.getState().reviewId).toBeNull();
    expect(useAppStore.getState().selectedTaskId).toBeNull();
    expect(useAppStore.getState().diffText).toBe('');
  });
});
