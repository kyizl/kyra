import type { VariantProps } from 'class-variance-authority';
import { cva } from 'class-variance-authority';

export { default as Badge } from './Badge.vue';

export const badgeVariants = cva(
  'inline-flex items-center justify-center gap-1 rounded-full px-2 py-0.5 text-[0.66rem] font-bold leading-none tracking-wide whitespace-nowrap',
  {
    variants: {
      variant: {
        default: 'badge-v2-default',
        accent: 'badge-v2-accent',
        outline: 'badge-v2-outline',
        nick: 'badge-v2-nick',
        danger: 'badge-v2-danger',
      },
    },
    defaultVariants: { variant: 'default' },
  },
);

export type BadgeVariants = VariantProps<typeof badgeVariants>;
