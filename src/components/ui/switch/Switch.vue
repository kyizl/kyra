<script setup lang="ts">
import { cn } from '@renderer/lib/utils';
import { SwitchRoot, SwitchThumb } from 'radix-vue';

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    class?: string;
    disabled?: boolean;
  }>(),
  { class: '', disabled: false },
);

const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>();
</script>

<template>
  <SwitchRoot
    v-press
    :checked="modelValue"
    :disabled="disabled"
    :class="cn('switch-v2 no-drag', props.class)"
    :data-state="modelValue ? 'checked' : 'unchecked'"
    @update:checked="emit('update:modelValue', $event)"
  >
    <SwitchThumb class="switch-v2-thumb" />
  </SwitchRoot>
</template>
