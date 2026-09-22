<script setup lang="ts">
import { cn } from '@renderer/lib/utils';
import { computed } from 'vue';

const props = withDefaults(defineProps<{ class?: string }>(), { class: '' });
const classes = computed(() => cn('card-v2 hybrid-glass noise-veil', props.class));
</script>

<template>
  <div :class="classes"><slot /></div>
</template>
