<script lang="ts"></script>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
const props = withDefaults(defineProps<{ name: string; size?: number }>(), { size: 20 });
const ttl = 7 * 24 * 60 * 60 * 1000;
const fallback = 'https://mc-heads.net/avatar/steve/32';

const cache = new Map<string, string>();
const inflight = new Map<string, Promise<string>>();

const avatarSrc = ref<string | null>(null);
const errored = ref(false);
const loading = ref(true);
const retried = ref(false);

function cacheKey(username: string): string {
  return `mchead_${username.toLowerCase()}`;
}

function getCached(username: string): string | null {
  const key = cacheKey(username);
  if (cache.has(key)) return cache.get(key)!;
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { url: string; ts: number };
    if (Date.now() - parsed.ts > ttl) {
      localStorage.removeItem(key);
      return null;
    }
    cache.set(key, parsed.url);
    return parsed.url;
  } catch {
    return null;
  }
}

function setCache(username: string, dataUrl: string): void {
  const key = cacheKey(username);
  cache.set(key, dataUrl);
  queueMicrotask(() => {
    try {
      localStorage.setItem(key, JSON.stringify({ url: dataUrl, ts: Date.now() }));
    } catch {}
  });
}

async function blobToDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(blob);
  });
}

async function fetchAvatar(username: string): Promise<string> {
  const key = cacheKey(username);
  if (inflight.has(key)) return inflight.get(key)!;
  const promise = (async () => {
    const url = `https://mc-heads.net/avatar/${encodeURIComponent(username)}/32`;
    try {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const blob = await response.blob();
      const dataUrl = await blobToDataUrl(blob);
      setCache(username, dataUrl);
      return dataUrl;
    } catch {
      return fallback;
    } finally {
      inflight.delete(key);
    }
  })();
  inflight.set(key, promise);
  return promise;
}

async function loadAvatar(username: string): Promise<void> {
  loading.value = true;
  errored.value = false;
  const cached = getCached(username);
  if (cached) {
    avatarSrc.value = cached;
    loading.value = false;
    return;
  }
  avatarSrc.value = await fetchAvatar(username);
  loading.value = false;
}

function handleError(): void {
  if (!retried.value) {
    retried.value = true;
    avatarSrc.value = fallback;
  } else {
    errored.value = true;
  }
}

watch(
  () => props.name,
  (name: string) => {
    retried.value = false;
    loadAvatar(name);
  },
);
onMounted(() => loadAvatar(props.name));

const initials = computed(() => props.name.slice(0, 2).toUpperCase());
const avatarColor = computed(() => {
  let hash = 0;
  for (let i = 0; i < props.name.length; i++) {
    hash = props.name.charCodeAt(i) + ((hash << 5) - hash);
  }
  return `hsl(${((hash % 360) + 360) % 360}, 48%, 36%)`;
});
</script>

<template>
  <div
    class="relative shrink-0 overflow-hidden"
    :style="{
      width: `${size}px`,
      height: `${size}px`,
      borderRadius: `${Math.max(2, size * 0.15)}px`,
      display: 'block',
      lineHeight: 0,
      boxShadow:
        '0 0 0 1px rgba(var(--color-accent-rgb), 0.22), 0 1px 4px rgba(0,0,0,0.35)',
    }"
  >
    <img
      v-if="avatarSrc && !errored"
      :src="avatarSrc"
      :alt="name"
      class="block h-full w-full"
      style="image-rendering: pixelated"
      @error="handleError"
    />
    <div
      v-else-if="loading"
      class="animate-shimmer h-full w-full"
    />
    <div
      v-else
      class="flex h-full w-full items-center justify-center font-bold text-white"
      :style="{ background: avatarColor, fontSize: `${Math.max(7, size * 0.36)}px` }"
    >
      {{ initials }}
    </div>
  </div>
</template>
