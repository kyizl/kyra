import type { VariantProps } from 'class-variance-authority';
import { cva } from 'class-variance-authority';

export { default as Button } from './Button.vue';

export const buttonVariants = cva(
  [
    'no-drag inline-flex shrink-0 items-center justify-center gap-1.5',
    'font-medium select-none whitespace-nowrap',
    'transition-[background-color,border-color,color,box-shadow,transform] duration-150',
    'ease-[cubic-bezier(0.25,0.46,0.45,0.94)]',
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-0',
    'disabled:pointer-events-none disabled:opacity-40',
    'active:scale-[0.97]',
  ].join(' '),
  {
    variants: {
      variant: {
        accent: 'btn-accent-v2',
        outline: 'btn-outline-v2',
        ghost: 'btn-ghost-v2',
        subtle: 'btn-subtle-v2',
        destructive: 'btn-destructive-v2',
        link: 'btn-link-v2',
        control: 'btn-control-v2',
        'control-icon': 'btn-control-v2-icon',
      },
      size: {
        default: 'h-8 rounded-lg px-3.5 text-[0.8rem]',
        sm: 'h-7 rounded-md px-2.5 text-[0.76rem]',
        lg: 'h-9 rounded-lg px-5 text-[0.85rem]',
        icon: 'h-7 w-7 rounded-md',
        'icon-sm': 'h-6 w-6 rounded-md',
      },
    },
    defaultVariants: {
      variant: 'ghost',
      size: 'default',
    },
  },
);

export type ButtonVariants = VariantProps<typeof buttonVariants>;
