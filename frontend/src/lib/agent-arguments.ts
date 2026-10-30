export function parseAgentArguments(input: string): string[] {
  const args: string[] = [];
  let value = '';
  let started = false;
  let quote: "'" | '"' | null = null;
  for (let index = 0; index < input.length; index++) {
    const character = input[index];
    if (quote === "'") {
      if (character === quote) quote = null;
      else value += character;
    } else if (quote === '"') {
      if (character === quote) quote = null;
      else if (character === '\\' && ['"', '\\', '$', '`', '\n'].includes(input[index + 1])) {
        const next = input[++index];
        if (next !== '\n') value += next;
      } else value += character;
    } else if (/\s/.test(character)) {
      if (started) {
        args.push(value);
        value = '';
        started = false;
      }
    } else if (character === "'" || character === '"') {
      quote = character;
      started = true;
    } else if (character === '\\') {
      if (index + 1 >= input.length)
        throw new Error('Complete the escaped argument or remove the trailing backslash.');
      const next = input[++index];
      if (next !== '\n') {
        value += next;
        started = true;
      }
    } else {
      value += character;
      started = true;
    }
  }
  if (quote) throw new Error('Close the quoted argument before saving.');
  if (started) args.push(value);
  return args;
}

export function formatAgentArguments(args: string[]): string {
  return args
    .map(value =>
      /^[a-zA-Z0-9_./:=+-]+$/.test(value) ? value : `'${value.replace(/'/g, "'\\''")}'`
    )
    .join(' ');
}

export function agentArgumentError(input: string): string | null {
  try {
    parseAgentArguments(input);
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}
