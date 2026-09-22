<script setup lang="ts">
import { Switch } from '@renderer/components/ui/switch';
import { computed } from 'vue';

const props = defineProps<{ label: string; description?: string; value: boolean }>();
const emit = defineEmits<{ update: [value: boolean] }>();

const model = computed({
  get: () => props.value,
  set: (value: boolean) => emit('update', value),
});
</script>

<template>
  <label class="toggle-row flex cursor-pointer items-center justify-between gap-4">
    <span class="flex min-w-0 flex-col">
      <span class="toggle-label font-medium">{{ label }}</span>
      <span
        v-if="description"
        class="toggle-desc mt-0.5 leading-relaxed"
        >{{ description }}</span
      >
    </span>
    <Switch v-model="model" />
  </label>
</template>

<style scoped>
.toggle-row {
  margin: 0 -0.75rem;
  padding: 0.85rem 0.75rem;
  transition: background 150ms ease;
}

.toggle-row:hover {
  background: rgba(var(--color-accent-rgb), 0.055);
}

.toggle-label {
  font-size: 0.92rem;
  color: var(--color-ink-2);
  transition: color 150ms var(--ease-out);
}

.toggle-desc {
  font-size: 0.78rem;
  color: var(--color-ink-3);
  line-height: 1.55;
}
</style>
