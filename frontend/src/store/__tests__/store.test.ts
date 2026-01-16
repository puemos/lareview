import { describe, it, expect, beforeEach } from 'vitest';
import { useAppStore } from '../index';
import type { Plan } from '../../types';

describe('AppStore - Plan Merging', () => {
  beforeEach(() => {
    useAppStore.getState().reset();
  });

  it('initializes with null plan', () => {
    expect(useAppStore.getState().plan).toBeNull();
  });

  it('sets initial plan correctly', () => {
    const initialPlan: Plan = {
      entries: [
        { content: 'Task 1', status: 'pending', priority: 'medium' },
        { content: 'Task 2', status: 'pending', priority: 'medium' },
      ],
    };

    useAppStore.getState().handleServerUpdate(initialPlan);
    expect(useAppStore.getState().plan?.entries).toHaveLength(2);
    expect(useAppStore.getState().plan?.entries[0].content).toBe('Task 1');
  });

  it('replaces existing plan with new entries (ACP compliance)', () => {
    const initialPlan: Plan = {
      entries: [{ content: 'Task 1', status: 'pending', priority: 'medium' }],
    };
    useAppStore.getState().handleServerUpdate(initialPlan);

    const update: Plan = {
      entries: [{ content: 'Task 2', status: 'pending', priority: 'medium' }],
    };
    useAppStore.getState().handleServerUpdate(update);

    const plan = useAppStore.getState().plan;
    expect(plan?.entries).toHaveLength(1);
    expect(plan?.entries[0].content).toBe('Task 2');
  });

  it('updates status by providing full plan update', () => {
    const initialPlan: Plan = {
      entries: [{ content: 'Verify auth flow.', status: 'pending', priority: 'medium' }],
    };
    useAppStore.getState().handleServerUpdate(initialPlan);

    // Agent sends full update with new status
    const update: Plan = {
      entries: [{ content: 'Verify auth flow', status: 'completed', priority: 'high' }],
    };
    useAppStore.getState().handleServerUpdate(update);

    const plan = useAppStore.getState().plan;
    expect(plan?.entries).toHaveLength(1);
    expect(plan?.entries[0].status).toBe('completed');
    expect(plan?.entries[0].priority).toBe('high');
  });

  it('replaces plan completely even if content is similar (no merging)', () => {
    const initialPlan: Plan = {
      entries: [{ content: '  Refactor code  ', status: 'pending' }],
    };
    useAppStore.getState().handleServerUpdate(initialPlan);

    const update: Plan = {
      entries: [{ content: 'refactor code.', status: 'in_progress' }],
    };
    useAppStore.getState().handleServerUpdate(update);

    const plan = useAppStore.getState().plan;
    expect(plan?.entries).toHaveLength(1);
    // The previous plan entry is gone, replaced by the new one
    expect(plan?.entries[0].content).toBe('refactor code.');
    expect(plan?.entries[0].status).toBe('in_progress');
  });
});
