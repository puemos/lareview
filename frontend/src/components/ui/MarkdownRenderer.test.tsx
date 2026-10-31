import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { MarkdownRenderer } from './MarkdownRenderer';
import type { DiffFile } from '../../types';

vi.mock('../Common/Mermaid', () => ({ Mermaid: () => <div>Diagram</div> }));
const file: DiffFile = {
  name: 'src/a.ts',
  new_path: 'src/a.ts',
  old_path: 'src/a.ts',
  hunks: [
    { old_start: 40, old_lines: 1, new_start: 40, new_lines: 1, content: '-unsafe();\n+safe();' },
  ],
};

describe('code links in review Markdown', () => {
  it('links plain locations and quoted source while leaving unverified suggestions alone', () => {
    const onOpen = vi.fn();
    render(
      <MarkdownRenderer codeContext={{ files: [file], onOpen }}>
        {
          'See src/a.ts:40 and `safe()`. Also a.ts:40. Try `invented()` instead.\n\n```ts\nsafe();\n```'
        }
      </MarkdownRenderer>
    );
    fireEvent.click(screen.getByRole('link', { name: 'src/a.ts:40' }));
    expect(onOpen).toHaveBeenLastCalledWith({ file, line: 40, side: 'new' });
    fireEvent.click(screen.getByRole('link', { name: 'safe()' }));
    expect(onOpen).toHaveBeenCalledTimes(2);
    fireEvent.click(screen.getByRole('link', { name: 'View src/a.ts:40 in changes' }));
    expect(onOpen).toHaveBeenCalledTimes(3);
    expect(screen.queryByRole('link', { name: 'invented()' })).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'a.ts:40' })).toHaveAttribute(
      'href',
      '#code/src%2Fa.ts?line=40&side=new'
    );
  });

  it('preserves explicit Markdown links without creating nested links', () => {
    const onOpen = vi.fn();
    const { container } = render(
      <MarkdownRenderer codeContext={{ files: [file], onOpen }}>
        {'[`src/a.ts`](src/a.ts#L40) and [docs](https://example.com "Documentation").'}
      </MarkdownRenderer>
    );
    expect(container.querySelector('a a')).toBeNull();
    fireEvent.click(screen.getByRole('link', { name: 'src/a.ts' }));
    expect(onOpen).toHaveBeenCalledWith({ file, line: 40, side: 'new' });
    expect(screen.getByRole('link', { name: 'docs' })).toHaveAttribute(
      'href',
      'https://example.com'
    );
    expect(screen.getByRole('link', { name: 'docs' })).toHaveAttribute('title', 'Documentation');
  });

  it('keeps normal Markdown rendering when review context is absent', () => {
    render(<MarkdownRenderer>{'`src/a.ts:40` and `safe()`.'}</MarkdownRenderer>);
    expect(screen.queryAllByRole('link')).toHaveLength(0);
  });
});
