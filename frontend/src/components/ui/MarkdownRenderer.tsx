import ReactMarkdown from 'react-markdown';
import { Children, isValidElement, useMemo } from 'react';
import type { Components } from 'react-markdown';
import { Mermaid } from '../Common/Mermaid';
import { useAppearanceStore } from '../../store/appearance';
import type { DiffFile, FeedbackAnchor } from '../../types';
import {
  codeLocationHref,
  codeReferencePlugin,
  locationFromHref,
  resolveCodeReference,
  type CodeLocation,
} from '../../lib/code-references';

interface MarkdownRendererProps {
  children: string;
  className?: string;
  codeContext?: {
    files: DiffFile[];
    anchor?: FeedbackAnchor | null;
    onOpen: (location: CodeLocation) => void;
  };
}

export function MarkdownRenderer({ children, className, codeContext }: MarkdownRendererProps) {
  const codeFontSize = useAppearanceStore(state => state.codeFontSize);
  const files = codeContext?.files;
  const anchor = codeContext?.anchor;
  const plugins = useMemo(
    () => (files ? [codeReferencePlugin(files, anchor)] : []),
    [files, anchor]
  );
  const codeLink = (location: CodeLocation, label: React.ReactNode) => (
    <a
      href={codeLocationHref(location)}
      title={`View ${location.file.name}:${location.line} in changes`}
      onClick={event => {
        event.preventDefault();
        codeContext?.onOpen(location);
      }}
      className="text-brand decoration-brand/50 hover:decoration-brand underline underline-offset-2"
    >
      {label}
    </a>
  );
  const components: Components = {
    a({ href, children: linkChildren, node: _node, ...props }) {
      const location =
        codeContext && href ? locationFromHref(href, codeContext.files, codeContext.anchor) : null;
      return location ? (
        codeLink(location, linkChildren)
      ) : (
        <a {...props} href={href}>
          {linkChildren}
        </a>
      );
    },
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
    pre({ children: blockChildren }) {
      const style = codeContext ? { fontSize: codeFontSize, lineHeight: 1.6 } : undefined;
      const code = Children.toArray(blockChildren).find(isValidElement);
      const props = code?.props as { children?: string; className?: string } | undefined;
      if (props?.className === 'language-mermaid') return <>{blockChildren}</>;
      const location =
        codeContext && props?.children
          ? resolveCodeReference(String(props.children), codeContext.files, codeContext.anchor)
          : null;
      return location ? (
        <div>
          <pre style={style}>{blockChildren}</pre>
          <div className="-mt-2 mb-4 text-xs">
            {codeLink(location, `View ${location.file.name}:${location.line} in changes`)}
          </div>
        </div>
      ) : (
        <pre style={style}>{blockChildren}</pre>
      );
    },
  };

  return (
    <div className={className || 'prose prose-invert prose-sm max-w-none'}>
      <ReactMarkdown components={components} remarkPlugins={plugins}>
        {children}
      </ReactMarkdown>
    </div>
  );
}
