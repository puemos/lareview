import React from 'react';
import { WindowDragRegion } from './WindowDragRegion';

interface HeaderProps {
  version: string;
}

export const Header: React.FC<HeaderProps> = ({ version }) => {
  return (
    <header className="border-border relative flex h-10 shrink-0 items-center justify-between border-b bg-gray-950 pr-4 pl-20 select-none">
      <WindowDragRegion className="absolute inset-0" />
      <div className="flex items-center gap-2 opacity-60 transition-opacity hover:opacity-100">
        <span className="font-display text-xs font-medium tracking-wide text-gray-400">
          lareview
        </span>
        <span className="rounded px-1.5 font-mono text-[10px] text-gray-600">{version}</span>
      </div>
    </header>
  );
};
