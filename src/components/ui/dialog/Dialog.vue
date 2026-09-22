<script setup lang="ts">
import { popEnter, popLeave, scrimEnter, scrimLeave } from '@renderer/lib/motion';
import { cn } from '@renderer/lib/utils';
import { useConfigStore } from '@renderer/store/config';
import {
  DialogContent,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from 'radix-vue';

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    titleVisuallyHidden?: boolean;
    class?: string;
    width?: string;
  }>(),
  { titleVisuallyHidden: false, class: '', width: '300px' },
);

const emit = defineEmits<{ 'update:open': [value: boolean] }>();

const config = useConfigStore();
</script>

<template>
  <DialogRoot
    :open="open"
    @update:open="emit('update:open', $event)"
  >
    <DialogPortal>
      <div class="dialog-v2-portal">
        <Transition
          :css="false"
          @enter="scrimEnter"
          @leave="scrimLeave"
        >
          <DialogOverlay
            v-if="open"
            class="dialog-v2-overlay"
            :style="{ borderRadius: config.roundedCorners ? '14px' : '0px' }"
            force-mount
          />
        </Transition>
        <Transition
          :css="false"
          @enter="popEnter"
          @leave="popLeave"
        >
          <DialogContent
            v-if="open"
            :style="{ width: props.width }"
            :class="cn('dialog-v2-content no-drag', props.class)"
            force-mount
          >
            <div class="dialog-v2-glow" />
            <DialogTitle :class="titleVisuallyHidden ? 'sr-only' : 'dialog-v2-title'">
              {{ title }}
            </DialogTitle>
            <div class="relative flex flex-col gap-3.5">
              <slot />
            </div>
          </DialogContent>
        </Transition>
      </div>
    </DialogPortal>
  </DialogRoot>
</template>
