import React from 'react';
import { toast } from 'sonner';
import { ICONS } from '../../constants/icons';
import { MarkdownRenderer } from '../ui/MarkdownRenderer';
import { useTauri } from '../../hooks/useTauri';
import type { UpdateInfo } from '../../hooks/useUpdateCheck';

const UPDATE_COMMAND = 'brew upgrade --cask lareview';

interface UpdateModalProps {
  isOpen: boolean;
  onClose: () => void;
  currentVersion: string;
  updateInfo: UpdateInfo;
}

export const UpdateModal: React.FC<UpdateModalProps> = ({
  isOpen,
  onClose,
  currentVersion,
  updateInfo,
}) => {
  const { copyToClipboard, openUrl } = useTauri();

  if (!isOpen) return null;

  const handleCopyCommand = async () => {
    await copyToClipboard(UPDATE_COMMAND);
    toast('Copied to clipboard', {
      description: 'Paste the command in your terminal to update.',
    });
  };

  return (
    <div className="animate-in fade-in fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm duration-200">
      <div
        className="bg-bg-primary border-border/50 animate-in zoom-in-95 flex w-full max-w-lg flex-col rounded-xl border shadow-2xl duration-200"
        onClick={e => e.stopPropagation()}
      >
        {/* Header */}
        <div className="border-border/50 bg-bg-secondary/30 flex items-center justify-between rounded-t-xl border-b px-5 py-4">
          <div className="flex items-center gap-2.5">
            <div className="bg-brand/10 text-brand rounded-md p-1.5">
              <ICONS.ARROW_UP size={18} />
            </div>
            <h3 className="text-text-primary text-sm font-semibold">Update Available</h3>
            <span className="bg-brand/10 text-brand rounded-full px-2 py-0.5 text-[10px] font-medium">
              v{currentVersion} → v{updateInfo.latestVersion}
            </span>
          </div>
          <button
            onClick={onClose}
            className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded p-1 transition-all"
            aria-label="Close"
          >
            <ICONS.ACTION_CLOSE size={18} />
          </button>
        </div>

        {/* Body */}
        <div className="custom-scrollbar max-h-[50vh] overflow-y-auto p-5">
          {/* What's New */}
          <div className="mb-5">
            <h4 className="text-text-primary mb-2 text-xs font-semibold">What&apos;s New</h4>
            {updateInfo.releaseNotes ? (
              <MarkdownRenderer className="prose prose-invert prose-sm max-w-none">
                {updateInfo.releaseNotes}
              </MarkdownRenderer>
            ) : (
              <p className="text-text-secondary text-xs italic">
                No release notes available for this version.
              </p>
            )}
          </div>

          {/* How to Update */}
          <div>
            <h4 className="text-text-primary mb-2 text-xs font-semibold">How to Update</h4>
            <div className="bg-bg-tertiary border-border/50 flex items-center justify-between rounded-lg border px-3 py-2">
              <code className="text-text-primary text-xs">{UPDATE_COMMAND}</code>
              <button
                onClick={handleCopyCommand}
                className="text-text-tertiary hover:text-text-primary hover:bg-bg-secondary rounded p-1 transition-all"
                aria-label="Copy command"
              >
                <ICONS.ACTION_COPY size={14} />
              </button>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="border-border/50 flex items-center justify-between border-t px-5 py-4">
          <button
            onClick={() => openUrl(updateInfo.releaseUrl)}
            className="text-text-secondary hover:text-text-primary flex items-center gap-1.5 text-xs transition-colors"
          >
            <ICONS.ACTION_OPEN_WINDOW size={14} />
            <span>View on GitHub</span>
          </button>
          <button
            onClick={handleCopyCommand}
            className="bg-brand hover:bg-brand/90 text-brand-fg flex items-center gap-2 rounded-lg px-4 py-2 text-xs font-semibold transition-all active:scale-[0.98]"
          >
            <ICONS.ACTION_COPY size={14} />
            Copy Update Command
          </button>
        </div>
      </div>
    </div>
  );
};
