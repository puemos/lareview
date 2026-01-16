import React, { useEffect, useRef, useState } from 'react';
import mermaid from 'mermaid';
import { ICONS } from '../../constants/icons';
import { Modal } from './Modal';

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

export const cleanMermaidChart = (chart: string): string => {
  let current = chart.trim();

  // Recursively unquote if it looks like a JSON-encoded string literal.
  // This handles cases where the agent double or triple encodes the string.
  let iterations = 0;
  while (
    current.startsWith('"') &&
    current.endsWith('"') &&
    current.length >= 2 &&
    iterations < 5
  ) {
    try {
      current = JSON.parse(current).trim();
      iterations++;
    } catch {
      break;
    }
  }

  // Replace literal \\n, \n (escaped), and carriage returns with actual newlines.
  let cleaned = current.replace(/\\n/g, '\n').replace(/\r/g, '').replace(/\\r/g, '');

  // Auto-quote labels containing parentheses if they are not already quoted.
  // This fixes common syntax errors without being too aggressive.
  // Pattern matches |label(with)parens| and replaces with |"label(with)parens"|
  cleaned = cleaned.replace(/(\|)([^"|\n]*?[()][^"|\n]*?)(\|)/g, '$1"$2"$3');

  return cleaned;
};

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
    // Only zoom when ctrl key is pressed (common pattern) or always?
    // User requested "mouse wheel/trackpad zoom". 
    // Usually trackpad pinch triggers wheel event with ctrlKey=true on some browsers, or just wheel.
    // To avoid interfering with page scroll (though fullscreen is fixed), let's just zoom.
    // In fullscreen we definitely want zoom. Inline maybe dangerous if it captures scroll.
    
    // For now, let's enable it always but prevent default only if we are zooming.
    // Actually, simply zooming on wheel is fine for this component if it's the specific intention.
    
    e.preventDefault();
    e.stopPropagation();
    
    const delta = -e.deltaY; 
    const factor = 0.1;
    const newScale = Math.min(Math.max(scale + (delta > 0 ? factor : -factor), 0.2), 5);
    setScale(newScale);
  };

  if (error) {
    return (
      <div className={`flex flex-col items-center justify-center gap-4 rounded-lg border border-red-500/20 bg-red-500/5 p-8 text-center ${className}`}>
        <div className="flex h-12 w-12 items-center justify-center rounded-full bg-red-500/10 text-red-500">
          <ICONS.ICON_WARNING size={24} />
        </div>
        <div className="space-y-1">
          <h3 className="text-sm font-medium text-red-400">Failed to render diagram</h3>
          <p className="text-xs text-text-disabled">The diagram content contains syntax errors</p>
        </div>
        
        <button
          onClick={() => setShowRaw(!showRaw)}
          className="text-xs font-medium text-text-secondary hover:text-text-primary underline"
        >
          {showRaw ? 'Hide Code' : 'Show Code'}
        </button>

        {showRaw && (
          <pre className="mt-2 text-left text-[10px] w-full max-w-lg overflow-auto rounded bg-bg-surface p-4 font-mono text-text-secondary border border-border">
            {chart}
          </pre>
        )}
      </div>
    );
  }

  if (!svg) {
    return null;
  }

  const controls = (
    <div className="absolute bottom-4 right-4 flex items-center gap-1 rounded-lg border border-border bg-bg-surface/90 p-1 shadow-lg backdrop-blur-sm z-50">
      <button
        onClick={handleZoomOut}
        className="rounded p-1.5 text-text-secondary hover:bg-bg-secondary hover:text-text-primary transition-colors"
        title="Zoom Out"
      >
        <ICONS.ACTION_ZOOM_OUT size={16} />
      </button>
      <span className="min-w-[3rem] text-center text-xs font-medium text-text-secondary">
        {Math.round(scale * 100)}%
      </span>
      <button
        onClick={handleZoomIn}
        className="rounded p-1.5 text-text-secondary hover:bg-bg-secondary hover:text-text-primary transition-colors"
        title="Zoom In"
      >
        <ICONS.ACTION_ZOOM_IN size={16} />
      </button>
      <div className="mx-1 h-4 w-px bg-border/50" />
      <button
        onClick={handleReset}
        className="rounded p-1.5 text-text-secondary hover:bg-bg-secondary hover:text-text-primary transition-colors"
        title="Reset View"
      >
        <ICONS.ACTION_REFRESH size={16} />
      </button>
      <button
        onClick={toggleFullscreen}
        className="rounded p-1.5 text-text-secondary hover:bg-bg-secondary hover:text-text-primary transition-colors"
        title={isFullscreen ? "Exit Fullscreen" : "Fullscreen"}
      >
        {isFullscreen ? <ICONS.ACTION_COLLAPSE size={16} /> : <ICONS.ACTION_EXPAND size={16} />}
      </button>
    </div>
  );

  const closeButton = isFullscreen ? (
    <button
      onClick={toggleFullscreen}
      className="absolute top-4 left-4 z-50 rounded-lg bg-bg-surface p-2 text-text-tertiary shadow-lg transition-colors hover:bg-bg-secondary hover:text-text-primary border border-border/50"
      title="Close Fullscreen"
    >
      <ICONS.ACTION_CLOSE size={20} />
    </button>
  ) : null;

  const renderContent = (fullscreen: boolean) => (
    <div
      className={`relative overflow-hidden bg-bg-secondary/20 select-none ${
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
          style={{
            transform: `translate(${position.x}px, ${position.y}px) scale(${scale})`,
            transition: isDragging ? 'none' : 'transform 0.1s ease-out',
            transformOrigin: 'center center',
          }}
          dangerouslySetInnerHTML={{ __html: svg }}
        />
      </div>
      {controls}
      {fullscreen && closeButton}
    </div>
  );

  if (isFullscreen) {
    return (
      <>
        {/* Placeholder to keep layout stability when docked if needed, or just render modal */}
         <div className={`mermaid-container flex justify-center items-center rounded-lg p-4 bg-bg-secondary/20 text-text-disabled text-xs border border-dashed border-border ${className}`}>
            Diagram is open in full screen
             <button onClick={toggleFullscreen} className="ml-2 text-brand hover:underline">Reopen here</button>
        </div>

        <Modal isOpen={isFullscreen} onClose={toggleFullscreen} hideCloseButton>
            {renderContent(true)}
        </Modal>
      </>
    );
  }

  return renderContent(false);
};
