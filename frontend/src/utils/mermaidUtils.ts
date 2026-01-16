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
