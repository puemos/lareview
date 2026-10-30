import { describe, expect, it } from 'vitest';
import { formatAgentArguments, parseAgentArguments } from './agent-arguments';

describe('agent argument editing', () => {
  it('keeps quoted whitespace and empty arguments', () => {
    expect(parseAgentArguments(`--model "review model" '' --stdio`)).toEqual([
      '--model',
      'review model',
      '',
      '--stdio',
    ]);
  });
  it('passes shell-looking values literally without expansion', () => {
    expect(parseAgentArguments('$HOME `uname` *.ts')).toEqual(['$HOME', '`uname`', '*.ts']);
  });
  it('round trips existing argument values including quotes, newlines, and backslashes', () => {
    const args = [
      '--acp',
      '',
      'review model',
      "it's quoted",
      '"double quotes"',
      'C:\\Program Files\\agent',
      'first\nsecond',
      '$HOME',
      '`echo hi`',
      '日本語',
    ];
    expect(parseAgentArguments(formatAgentArguments(args))).toEqual(args);
    expect(parseAgentArguments(formatAgentArguments([]))).toEqual([]);
  });
  it('rejects incomplete quoting and escapes before saving', () => {
    expect(() => parseAgentArguments(`--model 'unfinished`)).toThrow('Close the quoted argument');
    expect(() => parseAgentArguments('--model value\\')).toThrow('trailing backslash');
  });
});
