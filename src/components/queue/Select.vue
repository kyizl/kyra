<script setup lang="ts" generic="T extends string | number">
import type { SelectOption } from '@renderer/types/queue-safety';
import { dropEnter, dropLeave } from '@renderer/lib/motion';
import { onClickOutside } from '@vueuse/core';
import { ChevronDown } from 'lucide-vue-next';
import { computed, inject, onBeforeUnmount, ref, watch } from 'vue';

const props = defineProps<{
  modelValue: T;
  options: SelectOption<T>[];
}>();

const emit = defineEmits<{ 'update:modelValue': [value: T] }>();

const open = ref(false);
const rootRef = ref<HTMLElement | null>(null);
const dropdownOpen = inject<ReturnType<typeof ref<boolean>>>('dropdownOpen', ref(false));

watch(open, (isOpen) => {
  dropdownOpen.value = isOpen;
});

onClickOutside(rootRef, () => {
  open.value = false;
});

onBeforeUnmount(() => {
  if (open.value) dropdownOpen.value = false;
});

const selectedLabel = computed(
  () => props.options.find((option) => option.value === props.modelValue)?.label ?? '',
);

function select(value: T): void {
  emit('update:modelValue', value);
  open.value = false;
}
</script>

<template>
  <div
    ref="rootRef"
    class="qs-select"
  >
    <button
      type="button"
      class="qs-select-trigger no-drag"
      :class="{ 'qs-select-trigger--open': open }"
      @click="open = !open"
    >
      <span class="qs-select-label">{{ selectedLabel }}</span>
      <ChevronDown
        :size="11"
        class="qs-select-chevron"
        :class="{ 'qs-select-chevron--open': open }"
      />
    </button>
    <Transition
      :css="false"
      @enter="dropEnter"
      @leave="dropLeave"
    >
      <div
        v-if="open"
        class="qs-select-dropdown themed-scroll"
      >
        <button
          v-for="option in options"
          :key="option.value"
          type="button"
          class="qs-select-item"
          :class="{ 'qs-select-item--active': option.value === modelValue }"
          @click="select(option.value)"
        >
          {{ option.label }}
        </button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.qs-select {
  position: relative;
}

.qs-select-trigger {
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  width: 100%;
  height: 30px;
  padding: 0 9px;
  border-radius: var(--radius-sm);
  background: rgba(0, 0, 0, 0.18);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: var(--color-ink-2);
  font-family: var(--font-sans);
  font-size: 0.78rem;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition:
    border-color 150ms ease,
    background 150ms ease,
    color 150ms ease;
}

.qs-select-label {
  overflow: hidden;
  text-overflow: ellipsis;
}

.qs-select-trigger:hover,
.qs-select-trigger--open {
  border-color: rgba(var(--color-accent-rgb), 0.38);
  background: rgba(var(--color-accent-rgb), 0.08);
  color: var(--color-ink-1);
}

.qs-select-chevron {
  color: var(--color-ink-3);
  flex-shrink: 0;
  transition: transform 160ms cubic-bezier(0.34, 1.56, 0.64, 1);
}

.qs-select-chevron--open {
  transform: rotate(180deg);
}

.qs-select-dropdown {
  position: absolute;
  top: calc(100% + 5px);
  left: 0;
  z-index: 60;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 100%;
  max-height: 220px;
  overflow-y: auto;
  background: rgba(var(--color-bg-rgb), 0.94);
  backdrop-filter: blur(16px) saturate(150%);
  -webkit-backdrop-filter: blur(16px) saturate(150%);
  border: 1px solid rgba(var(--color-accent-rgb), 0.18);
  border-radius: var(--radius-md);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.6),
    0 0 0 1px rgba(var(--color-accent-rgb), 0.06);
  padding: 5px;
}

html.low-end .qs-select-dropdown {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  background: rgba(var(--color-bg-rgb), 0.98);
}

.qs-select-item {
  display: flex;
  align-items: center;
  width: 100%;
  padding: 7px 10px;
  border-radius: 6px;
  font-family: var(--font-sans);
  font-size: 0.78rem;
  font-weight: 500;
  line-height: 1;
  color: var(--color-ink-2);
  cursor: pointer;
  text-align: left;
  white-space: nowrap;
  transition:
    background 140ms cubic-bezier(0.25, 0.46, 0.45, 0.94),
    color 140ms ease;
}

.qs-select-item:hover {
  background: rgba(255, 255, 255, 0.06);
  color: var(--color-ink-1);
}

.qs-select-item--active {
  color: var(--color-accent-light);
  background: rgba(var(--color-accent-rgb), 0.12);
}

.qs-select-item--active:hover {
  background: rgba(var(--color-accent-rgb), 0.18);
}
</style>
