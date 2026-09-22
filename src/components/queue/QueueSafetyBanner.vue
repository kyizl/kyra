<script setup lang="ts">
import type { QueueVerdict } from '@renderer/types/queue-safety';
import { Button } from '@renderer/components/ui/button';
import { ShieldAlert, X } from 'lucide-vue-next';

defineProps<{ verdict: QueueVerdict }>();
const emit = defineEmits<{ dismiss: [] }>();

function capitalize(text: string): string {
  return text.length ? text.charAt(0).toUpperCase() + text.slice(1) : text;
}
</script>

<template>
  <div
    class="no-drag queue-safety-banner flex shrink-0 items-start justify-between gap-3 px-3 py-2"
  >
    <div class="flex min-w-0 items-start gap-2">
      <div class="queue-safety-icon">
        <ShieldAlert
          :size="16"
          style="color: var(--color-bad)"
        />
      </div>
      <div class="min-w-0">
        <div class="queue-safety-title">This lobby matches your safety conditions.</div>
        <div
          v-if="verdict.reasons.length"
          class="queue-safety-reasons"
        >
          <span
            v-for="(reason, i) in verdict.reasons"
            :key="i"
            class="queue-safety-reason-chip"
          >
            {{ capitalize(reason) }}
          </span>
        </div>
      </div>
    </div>
    <div class="flex shrink-0 items-center gap-1.5 self-center">
      <Button
        as="router-link"
        to="/queue"
        variant="control"
      >
        Review
      </Button>
      <Button
        variant="control-icon"
        size="icon-sm"
        style="
          width: 22px;
          height: 22px;
          border-radius: var(--radius-sm);
          background: rgba(255, 255, 255, 0.05);
          border: 1px solid rgba(255, 255, 255, 0.08);
          color: var(--color-ink-3);
        "
        @click="emit('dismiss')"
      >
        <X :size="9" />
      </Button>
    </div>
  </div>
</template>

<style scoped>
.queue-safety-banner {
  position: relative;
  overflow: hidden;
  margin-bottom: 10px;
  background: var(--panel-bg);
  backdrop-filter: var(--panel-blur);
  -webkit-backdrop-filter: var(--panel-blur);
  border-bottom: 1px solid rgba(248, 113, 113, 0.3);
}

.queue-safety-banner::before {
  content: '';
  position: absolute;
  inset: 0;
  width: 300%;
  left: -100%;
  background: linear-gradient(
    105deg,
    rgba(248, 113, 113, 0.09) 0%,
    rgba(248, 113, 113, 0.03) 50%,
    rgba(248, 113, 113, 0.06) 100%
  );
  will-change: transform;
  animation: banner-shimmer 6s ease-in-out infinite;
  animation-play-state: var(--anim-play-state, running);
  pointer-events: none;
}

.queue-safety-icon {
  display: flex;
  height: 32px;
  width: 32px;
  flex-shrink: 0;
  margin-top: 1px;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: rgba(248, 113, 113, 0.14);
  border: 1px solid rgba(248, 113, 113, 0.35);
}

.queue-safety-title {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--color-bad);
  line-height: 1.2;
  padding-top: 5px;
}

.queue-safety-reasons {
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem;
  margin-top: 0.4rem;
}

.queue-safety-reason-chip {
  font-size: 0.68rem;
  font-weight: 500;
  line-height: 1.4;
  color: rgba(248, 113, 113, 0.92);
  background: rgba(248, 113, 113, 0.1);
  border: 1px solid rgba(248, 113, 113, 0.22);
  border-radius: 999px;
  padding: 0.16rem 0.55rem;
  white-space: nowrap;
}

.banner-drop-enter-active,
.banner-drop-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.banner-drop-enter-from,
.banner-drop-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

@media (prefers-reduced-motion: reduce) {
  .queue-safety-banner::before {
    animation: none;
  }
}
</style>
