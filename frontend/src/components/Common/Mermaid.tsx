import React, { useEffect, useRef, useState } from 'react';
import mermaid from 'mermaid';

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

const cleanMermaidChart = (chart: string): string => {
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
  return current.replace(/\\n/g, '\n').replace(/\r/g, '').replace(/\\r/g, '');
};

export const Mermaid: React.FC<MermaidProps> = ({ chart, className }) => {
  const [svg, setSvg] = useState<string>('');
  const containerRef = useRef<HTMLDivElement>(null);
  const idRef = useRef<string>(`mermaid-${Math.random().toString(36).substring(2, 11)}`);

  useEffect(() => {
    const renderChart = async () => {
      if (!chart) return;

      try {
        const cleanedChart = cleanMermaidChart(chart);

        // Try to parse the chart first to validate syntax
        // This prevents Mermaid from rendering its own error UI
        try {
          await mermaid.parse(cleanedChart);
        } catch (parseError) {
          console.error('Mermaid syntax validation failed:', parseError);
          setSvg('');
          return;
        }

        const { svg } = await mermaid.render(idRef.current, cleanedChart);
        setSvg(svg);
      } catch (err) {
        console.error('Mermaid rendering failed:', err);
        setSvg('');
      }
    };

    renderChart();
  }, [chart]);

  if (!svg) {
    return null;
  }

  return (
    <div
      className={`mermaid-container bg-bg-secondary/20 custom-scrollbar flex justify-center overflow-auto rounded-lg p-4 ${className}`}
      ref={containerRef}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
};
