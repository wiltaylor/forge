import { JSX, splitProps } from 'solid-js';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';

export function Button(props: { variant?: Variant } & JSX.ButtonHTMLAttributes<HTMLButtonElement>) {
  const [local, rest] = splitProps(props, ['variant', 'class', 'children']);
  return (
    <button class={`fbtn fbtn-${local.variant ?? 'secondary'} ${local.class ?? ''}`} {...rest}>
      {local.children}
    </button>
  );
}
