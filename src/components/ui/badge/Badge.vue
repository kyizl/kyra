<script setup lang="ts">
import type { BadgeVariants } from '.';
import { cn } from '@renderer/lib/utils';
import { computed } from 'vue';
import { badgeVariants } from '.';

const props = withDefaults(
  defineProps<{
    variant?: BadgeVariants['variant'];
    class?: string;
  }>(),
  { variant: 'default', class: '' },
);

const classes = computed(() =>
  cn(badgeVariants({ variant: props.variant }), props.class),
);
</script>

<template>
  <span :class="classes"><slot /></span>
</template>
