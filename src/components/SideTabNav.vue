<script setup lang="ts">
import type { Component } from 'vue';
import { duration, ease, gsap, motionEnabled } from '@renderer/lib/motion';
import { useConfigStore } from '@renderer/store/config';
import { nextTick, onMounted, ref, watch } from 'vue';

export interface SideTab {
  id: string;
  label: string;
  icon: Component;
}

const props = defineProps<{ tabs: SideTab[]; modelValue: string }>();
const emit = defineEmits<{ 'update:modelValue': [id: string] }>();

const config = useConfigStore();
const navRef = ref<HTMLElement | null>(null);
const indicatorRef = ref<HTMLElement | null>(null);

function moveIndicator(immediate = false): void {
  void nextTick(() => {
    const nav = navRef.value;
    const indicator = indicatorRef.value;
    if (!nav || !indicator) return;

    const active = nav.querySelector<HTMLElement>('[data-active="true"]');
    if (!active) return;

    const y = active.offsetTop;
    const height = active.offsetHeight;

    if (immediate || !motionEnabled(config.lowEndMode)) {
      gsap.set(indicator, { y, height, opacity: 1 });
      return;
    }

    gsap.to(indicator, { y, height, duration: duration.slow, ease: ease.snap });
  });
}

watch(
  () => props.modelValue,
  () => moveIndicator(),
);
onMounted(() => moveIndicator(true));
</script>

<template>
  <nav
    ref="navRef"
    class="side-nav no-drag relative flex flex-col gap-1"
  >
    <div
      ref="indicatorRef"
      class="side-nav-indicator"
    />
    <button
      v-for="tab in tabs"
      :key="tab.id"
      v-press
      type="button"
      class="side-nav-item relative flex items-center gap-2.5 rounded-lg px-2 py-2 text-left"
      :data-active="modelValue === tab.id"
      :class="{ 'side-nav-item--active': modelValue === tab.id }"
      @click="emit('update:modelValue', tab.id)"
    >
      <span class="side-nav-icon relative flex shrink-0 items-center justify-center">
        <component
          :is="tab.icon"
          :size="12"
        />
      </span>
      <span class="side-nav-label relative">{{ tab.label }}</span>
    </button>
  </nav>
</template>
