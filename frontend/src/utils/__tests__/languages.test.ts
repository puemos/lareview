import { describe, it, expect } from 'vitest';
import { getLanguageFromPath } from '../languages';

describe('getLanguageFromPath', () => {
  it('maps Elixir source and script files to elixir', () => {
    expect(getLanguageFromPath('lib/my_app/greeter.ex')).toBe('elixir');
    expect(getLanguageFromPath('test/my_app/greeter_test.exs')).toBe('elixir');
    expect(getLanguageFromPath('mix.exs')).toBe('elixir');
  });

  it('maps Elixir templates to html', () => {
    expect(getLanguageFromPath('lib/my_app_web/live/page.html.heex')).toBe('html');
    expect(getLanguageFromPath('lib/my_app_web/templates/index.html.eex')).toBe('html');
  });

  it('is case insensitive', () => {
    expect(getLanguageFromPath('LIB/GREETER.EX')).toBe('elixir');
  });

  it('falls back to plaintext for unknown and extensionless paths', () => {
    expect(getLanguageFromPath('Dockerfile')).toBe('plaintext');
    expect(getLanguageFromPath('lib/thing.unknownext')).toBe('plaintext');
  });

  it('still maps existing languages', () => {
    expect(getLanguageFromPath('src/main.rs')).toBe('rust');
    expect(getLanguageFromPath('frontend/src/App.tsx')).toBe('typescript');
  });
});
