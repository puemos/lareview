import { useEffect, useState } from 'react';
import {
  APPEARANCE_DEFAULTS,
  CODE_FONT_SIZE,
  INTERFACE_SCALE,
  useAppearanceStore,
} from '../../store/appearance';

export function AppearanceSettings() {
  const { interfaceScale, codeFontSize, setInterfaceScale, setCodeFontSize } = useAppearanceStore();
  const [draftInterfaceScale, setDraftInterfaceScale] = useState(interfaceScale);
  useEffect(() => setDraftInterfaceScale(interfaceScale), [interfaceScale]);
  const hasChanges = draftInterfaceScale !== interfaceScale;
  const previewScale = draftInterfaceScale / interfaceScale;
  const isDefault =
    draftInterfaceScale === APPEARANCE_DEFAULTS.interfaceScale &&
    codeFontSize === APPEARANCE_DEFAULTS.codeFontSize;

  return (
    <div className="space-y-6">
      <div className="settings-section-heading">
        <h2 className="text-text-primary mb-2 text-lg font-semibold tracking-[-0.015em]">
          Appearance
        </h2>
        <p className="text-text-tertiary text-[0.8125rem] leading-6">
          Make LaReview comfortable to read.
        </p>
      </div>

      <div className="settings-card space-y-8">
        <div>
          <div className="flex items-center justify-between gap-4">
            <label htmlFor="interface-scale" className="text-text-primary text-sm font-medium">
              Interface text
            </label>
            <output htmlFor="interface-scale" className="text-text-secondary text-sm tabular-nums">
              {draftInterfaceScale}%
            </output>
          </div>
          <p id="interface-scale-description" className="text-text-tertiary mt-1 text-xs leading-5">
            Preview a size below, then save to apply it to the interface. Default: 100%.
          </p>
          <input
            id="interface-scale"
            type="range"
            {...INTERFACE_SCALE}
            value={draftInterfaceScale}
            onChange={event => setDraftInterfaceScale(Number(event.target.value))}
            aria-describedby="interface-scale-description"
            aria-valuetext={`${draftInterfaceScale}%`}
            className="mt-3 h-6 w-full"
          />
        </div>

        <div>
          <div className="flex items-center justify-between gap-4">
            <label htmlFor="code-font-size" className="text-text-primary text-sm font-medium">
              Code text
            </label>
            <output htmlFor="code-font-size" className="text-text-secondary text-sm tabular-nums">
              {codeFontSize} px
            </output>
          </div>
          <p id="code-font-size-description" className="text-text-tertiary mt-1 text-xs leading-5">
            Code and diffs in the input and review editors. Saves automatically. Default: 12 px.
          </p>
          <input
            id="code-font-size"
            type="range"
            {...CODE_FONT_SIZE}
            value={codeFontSize}
            onChange={event => setCodeFontSize(Number(event.target.value))}
            aria-describedby="code-font-size-description"
            aria-valuetext={`${codeFontSize} pixels`}
            className="mt-3 h-6 w-full"
          />
        </div>
      </div>

      <section aria-label="Text preview" className="settings-card space-y-4">
        <p className="text-text-tertiary text-xs font-medium">Preview</p>
        <div>
          <h3
            className="text-text-primary font-medium"
            style={{ fontSize: `${0.875 * previewScale}rem`, lineHeight: 1.5 }}
          >
            Review with confidence
          </h3>
          <p
            className="text-text-secondary mt-1"
            style={{ fontSize: `${0.8125 * previewScale}rem`, lineHeight: 1.7 }}
          >
            Read the changes, follow the plan, and share your feedback.
          </p>
        </div>
        <pre
          className="bg-bg-secondary text-text-secondary overflow-x-auto rounded-md p-4 font-mono"
          style={{ fontSize: codeFontSize, lineHeight: 1.7 }}
        >
          <code>{'function greet(name: string) {\n  return `Hello, ${name}`;\n}'}</code>
        </pre>
      </section>

      <div className="flex flex-wrap items-center gap-3">
        <button
          type="button"
          onClick={() => setInterfaceScale(draftInterfaceScale)}
          disabled={!hasChanges}
          className="bg-brand text-brand-fg rounded-md px-4 py-2 text-xs font-medium disabled:opacity-40"
        >
          Save changes
        </button>
        <button
          type="button"
          onClick={() => {
            setDraftInterfaceScale(APPEARANCE_DEFAULTS.interfaceScale);
            setCodeFontSize(APPEARANCE_DEFAULTS.codeFontSize);
          }}
          disabled={isDefault}
          className="settings-secondary-action"
        >
          Restore default sizes
        </button>
        <p role="status" className="text-text-tertiary text-xs">
          {hasChanges ? 'Interface size has unsaved changes.' : ''}
        </p>
      </div>
    </div>
  );
}
