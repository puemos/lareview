interface ToggleProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Toggle({ label, checked, onChange, disabled }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-label={label}
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className="product-toggle"
    >
      <span className="product-toggle-track" aria-hidden="true">
        <span />
      </span>
      <span aria-hidden="true">{checked ? 'Enabled' : 'Disabled'}</span>
    </button>
  );
}
