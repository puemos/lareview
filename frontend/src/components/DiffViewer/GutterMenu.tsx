import React from 'react';
import * as Popover from '@radix-ui/react-popover';
import { Chat, ArrowSquareOut } from '@phosphor-icons/react';

interface GutterMenuProps {
  position: { x: number; y: number } | null;
  onClose: () => void;
  onAddFeedback: () => void;
  onOpenInEditor: () => void;
}

export const GutterMenu: React.FC<GutterMenuProps> = ({
  position,
  onClose,
  onAddFeedback,
  onOpenInEditor,
}) => {
  if (!position) return null;

  // Create a virtual element for positioning based on coordinates
  const virtualElement = {
    getBoundingClientRect: () =>
      ({
        width: 0,
        height: 0,
        top: position.y,
        left: position.x,
        right: position.x,
        bottom: position.y,
        x: position.x,
        y: position.y,
      }) as DOMRect,
  };

  // We need a ref object for Radix
  const virtualRef = { current: virtualElement };

  return (
    <Popover.Root open={!!position} onOpenChange={open => !open && onClose()}>
      <Popover.Anchor virtualRef={virtualRef} />
      <Popover.Portal>
        <Popover.Content
          className="bg-bg-elevated border-border shadow-custom animate-in fade-in zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=top]:slide-in-from-bottom-2 z-50 min-w-[160px] rounded-lg border p-1 duration-100"
          side="bottom"
          align="start"
          sideOffset={5}
        >
          <div className="flex flex-col gap-0.5">
            <button
              onClick={() => {
                onAddFeedback();
                onClose();
              }}
              className="text-text-primary hover:bg-bg-tertiary flex items-center gap-2 rounded px-2 py-1.5 text-left text-xs transition-colors"
            >
              <Chat size={16} className="text-text-secondary" />
              Add Feedback
            </button>
            <button
              onClick={() => {
                onOpenInEditor();
                onClose();
              }}
              className="text-text-primary hover:bg-bg-tertiary flex items-center gap-2 rounded px-2 py-1.5 text-left text-xs transition-colors"
            >
              <ArrowSquareOut size={16} className="text-text-secondary" />
              Open in Editor
            </button>
          </div>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
};
