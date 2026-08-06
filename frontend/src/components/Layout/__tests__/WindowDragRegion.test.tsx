import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { WindowDragRegion } from '../WindowDragRegion';

const { startDragging } = vi.hoisted(() => ({
  startDragging: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ startDragging }),
}));

describe('WindowDragRegion', () => {
  it('starts a native window drag only for the primary mouse button', () => {
    const { container } = render(<WindowDragRegion className="h-6" />);
    const region = container.querySelector('[data-window-drag-region]');

    expect(region).not.toBeNull();
    fireEvent.mouseDown(region!, { button: 2 });
    expect(startDragging).not.toHaveBeenCalled();

    fireEvent.mouseDown(region!, { button: 0 });
    expect(startDragging).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});
