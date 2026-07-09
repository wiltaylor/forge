import type { Component, JSX } from 'solid-js';

/** Semantic tone used by badges, toasts, alerts, progress, etc. */
export type Tone = 'neutral' | 'accent' | 'success' | 'warning' | 'danger' | 'info';

/** Status tones (no accent) for dots and avatar status. */
export type StatusTone = 'neutral' | 'success' | 'warning' | 'danger' | 'info';

export type ControlSize = 'sm' | 'md' | 'lg';

/**
 * Shape of the icon components Forge accepts (lucide-solid components fit).
 * Icons are always passed in by the consumer — Forge has no icon dependency.
 */
export interface IconBaseProps {
  size?: number | string;
  strokeWidth?: number | string;
  [prop: string]: unknown;
}
export type IconComponent = Component<IconBaseProps>;

/** Generic option shape for Select / ListBox / Combobox / RadioGroup / ToggleGroup. */
export interface Option<T = string> {
  value: T;
  label: JSX.Element;
  disabled?: boolean;
  icon?: IconComponent;
}

/** Item shape for DropdownMenu / ContextMenu. */
export interface MenuItem {
  label?: JSX.Element;
  icon?: IconComponent;
  kbd?: string;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  onSelect?: () => void;
}

/** Item shape for the Command palette. */
export interface CommandItem {
  label: string;
  group?: string;
  icon?: IconComponent;
  kbd?: string;
  onSelect?: () => void;
}
