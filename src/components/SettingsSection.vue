<script setup lang="ts">
defineProps<{ title: string; description?: string }>();
</script>

<template>
  <div class="settings-section flex flex-col gap-0">
    <div class="section-heading">
      <span class="section-title">{{ title }}</span>
    </div>

    <p
      v-if="description"
      class="section-desc"
    >
      {{ description }}
    </p>

    <div class="section-card divide-subtle px-3">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.settings-section {
  position: relative;
  overflow: hidden;
  border-radius: 16px;
}

.section-heading {
  display: flex;
  align-items: center;
  min-height: 42px;
  padding: 0 0.9rem;

  background: rgba(255, 255, 255, 0.018);

  border-radius: 0 !important;
}

.section-title {
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--color-accent);
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.section-desc {
  margin: 0;
  padding: 0 0.9rem 0.75rem;

  font-size: 0.78rem;
  line-height: 1.55;
  color: var(--color-ink-3);

  background: rgba(255, 255, 255, 0.018);

  border-radius: 0 !important;
}

.section-card {
  display: flex;
  flex-direction: column;

  border-radius: 0 !important;
  border-top-left-radius: 0 !important;
  border-top-right-radius: 0 !important;
  border-bottom-left-radius: 0 !important;
  border-bottom-right-radius: 0 !important;
}
</style>
