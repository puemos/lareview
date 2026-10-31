import React, { useEffect, useRef, useState } from 'react';
import mermaid from 'mermaid';
import { ICONS } from '../../constants/icons';
import { cleanMermaidChart } from '../../utils/mermaidUtils';
import { Dialog } from './Dialog';

mermaid.initialize({
  startOnLoad: false,
  theme: 'dark',
  securityLevel: 'loose',
  fontFamily: 'GeistMono, Monaco, monospace',
});

interface MermaidProps {
  chart: string;
  className?: string;
}

export const Mermaid: React.FC<MermaidProps> = ({ chart, className }) => {
  const [svg, setSvg] = useState<string>('');
  const [scale, setScale] = useState(1);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showRaw, setShowRaw] = useState(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const idRef = useRef<string>(`mermaid-${Math.random().toString(36).substring(2, 11)}`);

  useEffect(() => {
    const renderChart = async () => {
      if (!chart) return;
      setError(null);

      try {
        const cleanedChart = cleanMermaidChart(chart);

        // Try to parse the chart first to validate syntax
        try {
          await mermaid.parse(cleanedChart);
        } catch (parseError) {
          console.error('Mermaid syntax validation failed:', parseError);
          setError(parseError instanceof Error ? parseError.message : 'Syntax error');
          setSvg('');
          return;
        }

        const { svg } = await mermaid.render(idRef.current, cleanedChart);
        setSvg(svg);
      } catch (err) {
        console.error('Mermaid rendering failed:', err);
        setError(err instanceof Error ? err.message : 'Rendering failed');
        setSvg('');
      }
    };

    renderChart();
  }, [chart]);

  const handleZoomIn = () => setScale(s => Math.min(s * 1.2, 5));
  const handleZoomOut = () => setScale(s => Math.max(s / 1.2, 0.2));
  const handleReset = () => {
    setScale(1);
    setPosition({ x: 0, y: 0 });
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    setIsDragging(true);
    setDragStart({ x: e.clientX - position.x, y: e.clientY - position.y });
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      setPosition({
        x: e.clientX - dragStart.x,
        y: e.clientY - dragStart.y,
      });
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  const toggleFullscreen = () => {
    if (isFullscreen) {
      setIsFullscreen(false);
      handleReset();
    } else {
      setIsFullscreen(true);
      handleReset();
    }
  };

  const handleWheel = (e: React.WheelEvent) => {
    if (!e.ctrlKey && !e.metaKey) return;
    e.preventDefault();
    e.stopPropagation();

    const delta = -e.deltaY;
    const factor = 0.1;
    const newScale = Math.min(Math.max(scale + (delta > 0 ? factor : -factor), 0.2), 5);
    setScale(newScale);
  };

  if (error) {
    return (
      <div
        className={`flex flex-col items-center justify-center gap-4 rounded-lg border border-red-500/20 bg-red-500/5 p-8 text-center ${className}`}
      >
        <div className="flex h-12 w-12 items-center justify-center rounded-full bg-red-500/10 text-red-500">
          <ICONS.ICON_WARNING size={24} />
        </div>
        <div className="space-y-1">
          <h3 className="text-sm font-medium text-red-400">Failed to render diagram</h3>
          <p className="text-text-disabled text-xs">The diagram content contains syntax errors</p>
        </div>

        <button
          onClick={() => setShowRaw(!showRaw)}
          className="text-text-secondary hover:text-text-primary text-xs font-medium underline"
        >
          {showRaw ? 'Hide Code' : 'Show Code'}
        </button>

        {showRaw && (
          <pre className="bg-bg-surface text-text-secondary border-border mt-2 w-full max-w-lg overflow-auto rounded border p-4 text-left font-mono text-[0.625rem]">
            {chart}
          </pre>
        )}
      </div>
    );
  }

  if (!svg) {
    return (
      <div
        role="status"
        className={`text-text-tertiary flex min-h-32 items-center justify-center text-xs ${className}`}
      >
        Rendering diagram…
      </div>
    );
  }

  const controls = (
    <div className="border-border bg-bg-surface/90 absolute right-4 bottom-4 z-50 flex items-center gap-1 rounded-lg border p-1 shadow-lg backdrop-blur-sm">
      <button
        onClick={handleZoomOut}
        className="text-text-secondary hover:bg-bg-secondary hover:text-text-primary rounded p-1.5 transition-colors"
        aria-label="Zoom out"
        title="Zoom out"
      >
        <ICONS.ACTION_ZOOM_OUT size={16} />
      </button>
      <span className="text-text-secondary min-w-[3rem] text-center text-xs font-medium">
        {Math.round(scale * 100)}%
      </span>
      <button
        onClick={handleZoomIn}
        className="text-text-secondary hover:bg-bg-secondary hover:text-text-primary rounded p-1.5 transition-colors"
        aria-label="Zoom in"
        title="Zoom in"
      >
        <ICONS.ACTION_ZOOM_IN size={16} />
      </button>
      <div className="bg-border/50 mx-1 h-4 w-px" />
      <button
        onClick={handleReset}
        className="text-text-secondary hover:bg-bg-secondary hover:text-text-primary rounded p-1.5 transition-colors"
        aria-label="Reset diagram view"
        title="Reset view"
      >
        <ICONS.ACTION_REFRESH size={16} />
      </button>
      <button
        onClick={toggleFullscreen}
        className="text-text-secondary hover:bg-bg-secondary hover:text-text-primary rounded p-1.5 transition-colors"
        aria-label={isFullscreen ? 'Exit fullscreen' : 'Expand diagram'}
        title={isFullscreen ? 'Exit fullscreen' : 'Expand diagram'}
      >
        {isFullscreen ? <ICONS.ACTION_COLLAPSE size={16} /> : <ICONS.ACTION_EXPAND size={16} />}
      </button>
    </div>
  );

  const renderContent = (fullscreen: boolean) => (
    <div
      className={`bg-bg-secondary/20 relative overflow-hidden select-none ${
        fullscreen ? 'h-full w-full' : `rounded-lg ${className}`
      }`}
      ref={containerRef}
      onWheel={handleWheel}
    >
      <div
        className={`flex h-full w-full items-center justify-center overflow-hidden ${
          isDragging ? 'cursor-grabbing' : 'cursor-grab'
        }`}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      >
        <div
          className="review-diagram-chart"
          style={{
            transform: `translate(${position.x}px, ${position.y}px) scale(${scale})`,
            transition: isDragging ? 'none' : 'transform 0.1s ease-out',
            transformOrigin: 'center center',
          }}
          dangerouslySetInnerHTML={{ __html: svg }}
        />
      </div>
      {controls}
    </div>
  );

  return (
    <>
      <div
        className={isFullscreen ? 'invisible h-full' : 'contents'}
        aria-hidden={isFullscreen || undefined}
      >
        {renderContent(false)}
      </div>
      <Dialog
        isOpen={isFullscreen}
        onClose={toggleFullscreen}
        title="Review diagram"
        description="Drag to pan. Use the controls or Control + scroll to zoom."
        className="product-diagram-dialog"
        bodyClassName="product-diagram-body"
      >
        {renderContent(true)}
      </Dialog>
    </>
  );
};
