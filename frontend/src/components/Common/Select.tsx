import React from 'react';
import * as SelectPrimitive from '@radix-ui/react-select';
import { ICONS } from '../../constants/icons';
import { Check } from '@phosphor-icons/react';

interface Option {
  value: string;
  label: string;
  icon?: React.ElementType;
  color?: string;
  className?: string;
}

interface SelectProps {
  value: string;
  onChange: (value: string) => void;
  options: Option[];
  placeholder?: string;
  disabled?: boolean;
  className?: string; // Class for the trigger button
  contentClassName?: string; // Class for the dropdown content
}

export const Select: React.FC<SelectProps> = ({
  value,
  onChange,
  options,
  placeholder = 'Select...',
  disabled = false,
  className = '',
  contentClassName = '',
}) => {
  const selectedOption = options.find(opt => opt.value === value);

  return (
    <SelectPrimitive.Root value={value} onValueChange={onChange} disabled={disabled}>
      <SelectPrimitive.Trigger
        className={`bg-bg-tertiary hover:bg-bg-secondary border-border/50 text-text-primary focus:border-brand focus:ring-brand/20 flex min-w-[100px] items-center justify-between gap-2 rounded border px-2.5 py-1.5 text-[11px] font-medium transition-all focus:ring-1 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50 ${className} `}
      >
        <span className="flex items-center gap-1.5 truncate">
          <SelectPrimitive.Value placeholder={placeholder}>
            {selectedOption && (
              <span className="text-text-primary flex items-center gap-1.5 overflow-hidden">
                {selectedOption.icon && (
                  <selectedOption.icon size={14} className={selectedOption.color} />
                )}
                <span className="truncate">{selectedOption.label}</span>
              </span>
            )}
          </SelectPrimitive.Value>
        </span>
        <SelectPrimitive.Icon className="text-text-tertiary shrink-0">
          <ICONS.CHEVRON_DOWN size={10} />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>

      <SelectPrimitive.Portal>
        <SelectPrimitive.Content
          className={`bg-bg-secondary border-border animate-in fade-in zoom-in-95 z-50 min-w-[var(--radix-select-trigger-width)] overflow-hidden rounded-md border shadow-xl duration-100 ease-out ${contentClassName} `}
          position="popper"
          sideOffset={4}
        >
          <SelectPrimitive.Viewport className="p-1">
            {options.map(option => (
              <SelectPrimitive.Item
                key={option.value}
                value={option.value}
                className={`text-text-secondary data-[highlighted]:bg-bg-tertiary data-[highlighted]:text-text-primary relative flex cursor-pointer items-center gap-2 rounded-[3px] px-3 py-2 pr-10 text-[11px] font-medium outline-none data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50 ${option.className || ''} `}
              >
                <div className="flex flex-1 items-center gap-2 overflow-hidden">
                  {option.icon && <option.icon size={14} className={option.color} />}
                  <SelectPrimitive.ItemText>{option.label}</SelectPrimitive.ItemText>
                </div>
                <SelectPrimitive.ItemIndicator className="text-brand absolute right-3 flex items-center justify-center">
                  <Check size={12} weight="bold" />
                </SelectPrimitive.ItemIndicator>
              </SelectPrimitive.Item>
            ))}
          </SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  );
};
