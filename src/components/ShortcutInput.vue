<script setup lang="ts">
import { Button } from '@renderer/components/ui/button';
import { X } from 'lucide-vue-next';
import { ref } from 'vue';

defineProps<{ label: string; value: string }>();
const emit = defineEmits<{ update: [value: string] }>();

const capturing = ref(false);

function onKey(event: KeyboardEvent): void {
  event.preventDefault();
  if (!capturing.value) return;
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) return;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('Control');
  if (event.shiftKey) parts.push('Shift');
  if (event.altKey) parts.push('Alt');
  if (event.metaKey) parts.push('Meta');
  parts.push(event.key.length === 1 ? event.key.toUpperCase() : event.key);
  emit('update', parts.join('+'));
  capturing.value = false;
}
</script>

<template>
  <div class="flex items-center justify-between gap-4 py-2.5">
    <span class="shortcut-label font-medium">{{ label }}</span>
    <div class="no-drag flex shrink-0 items-center gap-1.5">
      <div class="relative">
        <input
          :value="value || ''"
          readonly
          :placeholder="capturing ? 'Press keys…' : 'None'"
          class="input-field cursor-pointer font-mono select-none"
          :style="{
            fontSize: '0.72rem',
            width: '140px',
            paddingLeft: capturing ? '1.4rem' : undefined,
            color: capturing ? 'var(--color-accent-light)' : 'var(--color-ink-1)',
            borderColor: capturing ? 'rgba(var(--color-accent-rgb),0.55)' : undefined,
            boxShadow: capturing
              ? '0 0 0 2.5px rgba(var(--color-accent-rgb),0.16)'
              : undefined,
            background: capturing ? 'rgba(var(--color-accent-rgb),0.07)' : undefined,
          }"
          @click="capturing = true"
          @keydown.stop="onKey"
          @blur="capturing = false"
        />
        <span
          v-if="capturing"
          class="animate-pulse-dot absolute top-1/2 left-2 h-1.5 w-1.5 -translate-y-1/2 rounded-full"
          style="
            background: var(--color-accent-light);
            box-shadow: 0 0 6px var(--color-accent-glow);
          "
        />
      </div>
      <Button
        v-if="value"
        variant="ghost"
        size="icon-sm"
        @click="emit('update', '')"
      >
        <X :size="10" />
      </Button>
    </div>
  </div>
</template>

<style scoped>
.shortcut-label {
  font-size: 0.82rem;
  color: var(--color-ink-2);
}
</style>
