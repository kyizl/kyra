<script setup lang="ts">
import { Clock, Fingerprint, ShieldCheck, Tag } from 'lucide-vue-next';
</script>

<template>
  <div class="usage-disclosure">
    <span class="section-label">Usage statistics sent while the app is running</span>

    <div class="data-list">
      <div class="data-item">
        <Fingerprint
          :size="15"
          class="data-icon"
        />
        <span class="data-name">Discord User ID</span>
      </div>
      <div class="data-item">
        <Tag
          :size="15"
          class="data-icon"
        />
        <span class="data-name">App build and version</span>
      </div>
      <div class="data-item">
        <Clock
          :size="15"
          class="data-icon"
        />
        <span class="data-name">When sessions start, run and end</span>
      </div>
    </div>

    <div class="privacy-note">
      <ShieldCheck
        :size="14"
        class="privacy-icon"
      />
      <p>
        No IP address, device details, operating system, in-game data or anything you type
        is sent or stored. Session details are kept for 90 days.
      </p>
    </div>
  </div>
</template>

<style scoped>
.usage-disclosure {
  width: 100%;
  margin-bottom: 16px;
  text-align: left;
}

.section-label {
  display: block;
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--modal-ink-faint);
  margin-bottom: 8px;
}

.data-list {
  display: flex;
  flex-direction: column;
  background: var(--modal-fill);
  border: 1px solid var(--modal-line);
  border-radius: 10px;
  overflow: hidden;
}

.data-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
}

.data-item + .data-item {
  border-top: 1px solid var(--modal-line);
}

.data-icon {
  flex-shrink: 0;
  color: var(--modal-ink-muted);
}

.data-name {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--modal-ink);
}

.privacy-note {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-top: 12px;
}

.privacy-icon {
  flex-shrink: 0;
  margin-top: 1px;
  color: var(--modal-ink-faint);
}

.privacy-note p {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--modal-ink-muted);
}
</style>
