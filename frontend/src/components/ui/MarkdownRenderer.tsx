import ReactMarkdown from 'react-markdown';
import type { Components } from 'react-markdown';
import { Mermaid } from '../Common/Mermaid';

interface MarkdownRendererProps {
  children: string;
  className?: string;
}

export function MarkdownRenderer({ children, className }: MarkdownRendererProps) {
  const components: Components = {
    code({ className: codeClassName, children: codeChildren }) {
      const match = /language-(\w+)/.exec(codeClassName || '');
      if (match && match[1] === 'mermaid') {
        return (
          <Mermaid
            chart={String(codeChildren || '').replace(/\n$/, '')}
            className="my-4 max-h-64"
          />
        );
      }
      return <code className={codeClassName}>{codeChildren}</code>;
    },
  };

  return (
    <div className={className || 'prose prose-invert prose-sm max-w-none'}>
      <ReactMarkdown components={components}>{children}</ReactMarkdown>
    </div>
  );
}
