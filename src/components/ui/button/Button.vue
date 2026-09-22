<script setup lang="ts">
import type { ClassValue } from 'clsx';
import type { ButtonVariants } from '.';
import { cn } from '@renderer/lib/utils';
import { computed } from 'vue';
import { buttonVariants } from '.';

const props = withDefaults(
  defineProps<{
    variant?: ButtonVariants['variant'];
    size?: ButtonVariants['size'];
    as?: string;
    class?: ClassValue;
    magnetic?: number;
  }>(),
  {
    variant: 'ghost',
    size: 'default',
    as: 'button',
    class: '',
    magnetic: 0,
  },
);

const classes = computed(() =>
  cn(buttonVariants({ variant: props.variant, size: props.size }), props.class),
);
</script>

<template>
  <component
    :is="as"
    v-press="magnetic || undefined"
    :class="classes"
    v-bind="as === 'button' ? { type: 'button' } : {}"
  >
    <slot />
  </component>
</template>
