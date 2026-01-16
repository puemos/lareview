import { describe, it, expect } from 'vitest';
import { cleanMermaidChart } from './Mermaid';

describe('cleanMermaidChart', () => {
  it('unquotes double-encoded JSON strings', () => {
    const chart = '"flowchart TD\\nA --> B"';
    expect(cleanMermaidChart(chart)).toBe('flowchart TD\nA --> B');
  });

  it('unquotes triple-encoded JSON strings', () => {
    const chart = '"\\"flowchart TD\\\\nA --> B\\""';
    expect(cleanMermaidChart(chart)).toBe('flowchart TD\nA --> B');
  });

  it('handles regular chart strings', () => {
    const chart = 'flowchart TD\nA --> B';
    expect(cleanMermaidChart(chart)).toBe('flowchart TD\nA --> B');
  });

  it('sanitizes unquoted labels with parentheses', () => {
    const chart = 'flowchart TD\nRefreshTrigger --> |throttleTime(700ms)| Slice';
    expect(cleanMermaidChart(chart)).toBe('flowchart TD\nRefreshTrigger --> |"throttleTime(700ms)"| Slice');
  });

  it('does not double-quote already quoted labels', () => {
    const chart = 'flowchart TD\nRefreshTrigger --> |"throttleTime(700ms)"| Slice';
    expect(cleanMermaidChart(chart)).toBe('flowchart TD\nRefreshTrigger --> |"throttleTime(700ms)"| Slice');
  });

  it('handles multiple unquoted labels with parentheses', () => {
    const chart = 'flowchart TD\nA --> |f(x)| B\nB --> |g(y)| C';
    expect(cleanMermaidChart(chart)).toBe('flowchart TD\nA --> |"f(x)"| B\nB --> |"g(y)"| C');
  });

  it('handles labels without parentheses', () => {
    const chart = 'flowchart TD\nA --> |simple label| B';
    expect(cleanMermaidChart(chart)).toBe('flowchart TD\nA --> |simple label| B');
  });

  it('handles arrow types with labels', () => {
    const cases = [
      ['A --- |label(x)| B', 'A --- |"label(x)"| B'],
      ['A -.-> |label(x)| B', 'A -.-> |"label(x)"| B'],
      ['A ==> |label(x)| B', 'A ==> |"label(x)"| B'],
      ['A -- label(x) --> B', 'A -- label(x) --> B'], // Pattern only target |label| for now as it's the most common failure point
    ];
    
    for (const [input, expected] of cases) {
      expect(cleanMermaidChart(input)).toBe(expected);
    }
  });

  it('handles complex labels with other special characters if they have parens', () => {
    const chart = 'flowchart TD\nA --> |label (with) [brackets] and {braces}| B';
    expect(cleanMermaidChart(chart)).toBe('flowchart TD\nA --> |"label (with) [brackets] and {braces}"| B');
  });
  
  it('does not mess up sequence diagrams without edge labels', () => {
    const chart = 'sequenceDiagram\nAlice->>Bob: Hello(world)';
    expect(cleanMermaidChart(chart)).toBe('sequenceDiagram\nAlice->>Bob: Hello(world)');
  });
});
