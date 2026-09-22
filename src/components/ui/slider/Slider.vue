<script setup lang="ts">
import { cn } from '@renderer/lib/utils';
import { SliderRange, SliderRoot, SliderThumb, SliderTrack } from 'radix-vue';
import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
    class?: string;
  }>(),
  { min: 0, max: 100, step: 1, class: '' },
);

const emit = defineEmits<{ 'update:modelValue': [value: number] }>();

const localValue = computed<number[]>({
  get: () => [props.modelValue],
  set: (value) => {
    if (value.length) emit('update:modelValue', value[0]);
  },
});
</script>

<template>
  <SliderRoot
    v-model="localValue"
    :min="min"
    :max="max"
    :step="step"
    :class="cn('slider-v2 no-drag', props.class)"
  >
    <SliderTrack class="slider-v2-track">
      <SliderRange class="slider-v2-range" />
    </SliderTrack>
    <SliderThumb
      v-press="8"
      class="slider-v2-thumb"
    >
      <span class="slider-v2-thumb-core" />
    </SliderThumb>
  </SliderRoot>
</template>
