import React from 'react';
import { Gear, User, Terminal, Database, Asterisk, CaretRight } from '@phosphor-icons/react';
import { VcsSkeleton } from './SettingsSkeleton';

export const SettingsPageSkeleton: React.FC = () => {
  return (
    <div className="bg-bg-primary flex h-full flex-col">
      {/* Header */}
      <div className="border-border flex h-12 shrink-0 items-center gap-3 border-b px-6">
        <Asterisk size={18} weight="fill" className="text-brand" />
        <h1 className="font-display text-text-primary text-sm font-medium tracking-wide">
          Settings
        </h1>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <div className="border-border bg-bg-secondary/30 flex w-[240px] flex-col border-r pt-4">
          <nav className="flex-1 space-y-1 px-3">
            <div className="bg-bg-tertiary text-text-primary flex w-full items-center gap-3 rounded-md px-3 py-2 text-xs font-medium shadow-sm">
              <span className="text-brand">
                <Gear size={14} />
              </span>
              <span>VCS Integration</span>
              <CaretRight size={12} className="text-text-tertiary ml-auto" />
            </div>

            <div className="text-text-secondary flex w-full items-center gap-3 rounded-md px-3 py-2 text-xs font-medium">
              <span className="text-text-tertiary">
                <Terminal size={14} />
              </span>
              <span>CLI Tools</span>
            </div>

            <div className="text-text-secondary flex w-full items-center gap-3 rounded-md px-3 py-2 text-xs font-medium">
              <span className="text-text-tertiary">
                <Database size={14} />
              </span>
              <span>Editor</span>
            </div>

            <div className="pt-3 pb-1">
              <div className="bg-border/50 mx-2 h-px" />
            </div>

            <div className="text-text-secondary flex w-full items-center gap-3 rounded-md px-3 py-2 text-xs font-medium">
              <span className="text-text-tertiary">
                <User size={14} />
              </span>
              <span>Agents</span>
            </div>
          </nav>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto">
          <div className="animate-fade-in max-w-3xl space-y-8 p-8 md:p-12">
            <div className="mb-6">
              <div className="bg-bg-secondary mb-2 h-7 w-48 animate-pulse rounded" />
              <div className="bg-bg-secondary h-4 w-96 animate-pulse rounded" />
            </div>
            <VcsSkeleton />
          </div>
        </div>
      </div>
    </div>
  );
};
