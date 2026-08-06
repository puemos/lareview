import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { LiveActivityFeed, type ProgressMessage } from '../LiveActivityFeed';

const message = (id: number): ProgressMessage => ({
  id,
  type: 'log',
  message: `Activity ${id}`,
  timestamp: id,
});

describe('LiveActivityFeed', () => {
  beforeEach(() => {
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      callback(0);
      return 1;
    });
    vi.stubGlobal('cancelAnimationFrame', vi.fn());
  });

  it('follows new activity until the user scrolls away from the bottom', () => {
    const { rerender } = render(
      <LiveActivityFeed messages={[message(1)]} isRunning={true} />
    );
    const scroll = screen.getByTestId('activity-scroll');
    Object.defineProperties(scroll, {
      scrollHeight: { configurable: true, value: 300 },
      clientHeight: { configurable: true, value: 100 },
    });
    scroll.scrollTop = 200;

    rerender(<LiveActivityFeed messages={[message(1), message(2)]} isRunning={true} />);
    expect(scroll.scrollTop).toBe(300);

    scroll.scrollTop = 100;
    fireEvent.scroll(scroll);
    Object.defineProperty(scroll, 'scrollHeight', { configurable: true, value: 400 });
    rerender(
      <LiveActivityFeed messages={[message(1), message(2), message(3)]} isRunning={true} />
    );

    expect(scroll.scrollTop).toBe(100);
    expect(screen.getByRole('button', { name: 'Jump to latest' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Jump to latest' }));
    expect(scroll.scrollTop).toBe(400);

    Object.defineProperty(scroll, 'scrollHeight', { configurable: true, value: 500 });
    rerender(
      <LiveActivityFeed
        messages={[message(1), message(2), message(3), message(4)]}
        isRunning={true}
      />
    );
    expect(scroll.scrollTop).toBe(500);
  });
});
