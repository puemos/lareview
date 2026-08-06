import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { PlanOverview } from '../PlanOverview';

describe('PlanOverview', () => {
  it('shows one persistent empty state without a collapse control', () => {
    render(
      <PlanOverview
        items={[]}
        emptyMessage="The agent's plan will appear here once analysis begins."
      />
    );

    expect(screen.getByText('Plan')).toBeInTheDocument();
    expect(
      screen.getByText("The agent's plan will appear here once analysis begins.")
    ).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});
