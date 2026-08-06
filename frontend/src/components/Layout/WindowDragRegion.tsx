import { useCallback } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface WindowDragRegionProps {
  className: string;
}

export const WindowDragRegion: React.FC<WindowDragRegionProps> = ({ className }) => {
  const startDragging = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;

    event.preventDefault();
    void getCurrentWindow()
      .startDragging()
      .catch(error => console.error('Failed to start window drag:', error));
  }, []);

  return (
    <div
      data-window-drag-region
      aria-hidden="true"
      className={className}
      onMouseDown={startDragging}
    />
  );
};
