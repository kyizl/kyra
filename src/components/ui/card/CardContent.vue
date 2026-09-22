<script setup lang="ts">
import { cn } from '@renderer/lib/utils';

const props = withDefaults(defineProps<{ class?: string }>(), { class: '' });
</script>

<template>
  <div :class="cn('flex flex-col gap-3 p-3.5', props.class)"><slot /></div>
</template>
