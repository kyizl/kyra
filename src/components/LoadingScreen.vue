<script setup lang="ts">
import { useConfigStore } from '@renderer/store/config';
import { onMounted, onUnmounted, ref } from 'vue';

const emit = defineEmits<{ done: [] }>();
const config = useConfigStore();

const fading = ref(false);
const videoRef = ref<HTMLVideoElement | null>(null);
const videoSrc = config.lowEndMode ? '/loading-lo.mp4' : '/loading.mp4';

let safetyTimer: ReturnType<typeof setTimeout> | null = null;
let rafId: number | null = null;

function finish(): void {
  if (fading.value) return;
  fading.value = true;
  setTimeout(emit, 350, 'done');
}

function startPlayback(video: HTMLVideoElement): void {
  video.play().catch(() => finish());
}

onMounted(() => {
  const video = videoRef.value;
  if (!video) {
    safetyTimer = setTimeout(finish, 1800);
    return;
  }

  video.addEventListener('ended', finish, { once: true });
  video.addEventListener('error', finish, { once: true });
  safetyTimer = setTimeout(finish, 14_000);

  rafId = requestAnimationFrame(() => {
    rafId = requestAnimationFrame(() => {
      if (!video) return;
      if (video.readyState >= 3) {
        startPlayback(video);
      } else {
        video.addEventListener('canplaythrough', () => startPlayback(video), {
          once: true,
        });
      }
    });
  });
});

onUnmounted(() => {
  if (safetyTimer) clearTimeout(safetyTimer);
  if (rafId !== null) cancelAnimationFrame(rafId);
  const video = videoRef.value;
  if (video) {
    video.removeEventListener('ended', finish);
    video.removeEventListener('error', finish);
    video.pause();
    video.src = '';
    video.load();
  }
});
</script>

<template>
  <div
    class="loading-root fixed inset-0 overflow-hidden"
    :style="{
      zIndex: 10000,
      background: 'var(--color-bg)',
      opacity: fading ? 0 : 1,
      transition: 'opacity 0.35s ease-in-out',
      pointerEvents: fading ? 'none' : 'all',
      borderRadius: config.roundedCorners ? '14px' : '0px',
    }"
  >
    <video
      ref="videoRef"
      :src="videoSrc"
      muted
      playsinline
      preload="metadata"
      disablepictureinpicture
      :loop="false"
      class="loading-video block h-full w-full object-cover"
    />
  </div>
</template>

<style scoped>
.loading-root {
  transform: translateZ(0);
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
  isolation: isolate;
  contain: strict;
}

.loading-video {
  transform: translateZ(0);
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
  contain: strict;
  display: block;
  image-rendering: auto;
  object-fit: cover;
}
</style>
