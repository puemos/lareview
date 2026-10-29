import { useRef } from 'react';
import { MagnifyingGlass, X } from '@phosphor-icons/react';

interface SearchFieldProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export function SearchField({ label, value, onChange, placeholder }: SearchFieldProps) {
  const input = useRef<HTMLInputElement>(null);
  const clear = () => {
    onChange('');
    input.current?.focus();
  };
  return (
    <div className="product-search">
      <MagnifyingGlass size={15} aria-hidden="true" />
      <input
        ref={input}
        type="text"
        role="searchbox"
        aria-label={label}
        value={value}
        placeholder={placeholder ?? label}
        onChange={event => onChange(event.target.value)}
        onKeyDown={event => {
          if (event.key === 'Escape' && value) {
            event.preventDefault();
            clear();
          }
        }}
      />
      {value && (
        <button type="button" aria-label={`Clear ${label.toLowerCase()}`} onClick={clear}>
          <X size={13} />
        </button>
      )}
    </div>
  );
}
