<script setup lang="ts">
import type {
  MatchMode,
  QueueMetric,
  QueueRule,
  ValueComparator,
} from '@renderer/types/queue-safety';
import Select from '@renderer/components/queue/Select.vue';
import { Button } from '@renderer/components/ui/button';
import { Switch } from '@renderer/components/ui/switch';
import { useConfigStore } from '@renderer/store/config';
import { usePlayersStore } from '@renderer/store/players';
import {
  comparators,
  createExcludedPlayer,
  createRule,
  evaluateQueueSafety,
  isBooleanMetric,
  isUsernameMetric,
  QueueMetricOptions,
} from '@renderer/types/queue-safety';
import { Check, Plus, ShieldAlert, ShieldCheck, Trash2 } from 'lucide-vue-next';
import { computed, ref } from 'vue';

const config = useConfigStore();
const players = usePlayersStore();
const queueSafety = computed(() => config.queueSafety);
const verdict = computed(() => evaluateQueueSafety(queueSafety.value, players.players));
const pendingRuleIds = ref<Set<string>>(new Set());
const pendingExcludedIds = ref<Set<string>>(new Set());

const matchModes: { value: MatchMode; label: string; description: string }[] = [
  {
    value: 'any',
    label: 'Any',
    description: 'Flag the lobby as soon as one condition matches.',
  },
  {
    value: 'all',
    label: 'All',
    description: 'Flag the lobby only once every condition matches together.',
  },
];

function addRule(): void {
  const rule = createRule();
  queueSafety.value.rules.push(rule);
  pendingRuleIds.value = new Set(pendingRuleIds.value).add(rule.id);
}

function confirmRule(ruleId: string): void {
  if (!pendingRuleIds.value.has(ruleId)) return;

  const next = new Set(pendingRuleIds.value);
  next.delete(ruleId);
  pendingRuleIds.value = next;
}

function removeRule(ruleId: string): void {
  queueSafety.value.rules = queueSafety.value.rules.filter((rule) => rule.id !== ruleId);

  if (pendingRuleIds.value.has(ruleId)) {
    const next = new Set(pendingRuleIds.value);
    next.delete(ruleId);
    pendingRuleIds.value = next;
  }
}

function addExcludedPlayer(): void {
  const excludedPlayer = createExcludedPlayer();
  queueSafety.value.excludedPlayers.push(excludedPlayer);
  pendingExcludedIds.value = new Set(pendingExcludedIds.value).add(excludedPlayer.id);
}

function confirmExcludedPlayer(excludedPlayerId: string): void {
  if (!pendingExcludedIds.value.has(excludedPlayerId)) return;

  const next = new Set(pendingExcludedIds.value);
  next.delete(excludedPlayerId);
  pendingExcludedIds.value = next;
}

function removeExcludedPlayer(excludedPlayerId: string): void {
  queueSafety.value.excludedPlayers = queueSafety.value.excludedPlayers.filter(
    (excludedPlayer) => excludedPlayer.id !== excludedPlayerId,
  );

  if (pendingExcludedIds.value.has(excludedPlayerId)) {
    const next = new Set(pendingExcludedIds.value);
    next.delete(excludedPlayerId);
    pendingExcludedIds.value = next;
  }
}

function onMetricChange(rule: QueueRule, metric: QueueMetric): void {
  rule.metric = metric;

  if (isUsernameMetric(metric)) {
    rule.minPlayers = 1;
  }
}

function clampMinPlayers(rule: QueueRule): void {
  const clamped = Math.min(16, Math.max(1, Math.round(rule.minPlayers) || 1));

  if (rule.minPlayers !== clamped) {
    rule.minPlayers = clamped;
  }
}

function clampValue(rule: QueueRule): void {
  if (!Number.isFinite(rule.value)) {
    rule.value = 0;
  }
}

function capitalize(text: string): string {
  return text.length ? text.charAt(0).toUpperCase() + text.slice(1) : text;
}
</script>

<template>
  <div class="queue-outer">
    <div class="no-drag queue-banner">
      <div class="queue-banner-inner">
        <div class="queue-banner-copy">
          <div
            class="queue-banner-icon"
            :style="{
              background: verdict.unsafe
                ? 'rgba(248, 113, 113, 0.14)'
                : 'rgba(var(--color-accent-rgb), 0.12)',
              border: verdict.unsafe
                ? '1px solid rgba(248, 113, 113, 0.35)'
                : '1px solid rgba(var(--color-accent-rgb), 0.28)',
            }"
          >
            <ShieldAlert
              v-if="verdict.unsafe"
              :size="16"
              style="color: var(--color-bad)"
            />
            <ShieldCheck
              v-else
              :size="16"
              style="color: var(--color-accent-light)"
            />
          </div>

          <div class="queue-banner-text">
            <div class="queue-banner-title">Queue Safety</div>
            <div class="queue-banner-desc">
              Auto-flag lobbies that match your risk conditions.
            </div>
          </div>
        </div>

        <Switch
          class="no-drag"
          :model-value="queueSafety.enabled"
          @update:model-value="queueSafety.enabled = $event"
        />
      </div>
    </div>

    <main class="queue-content themed-scroll">
      <section class="queue-section">
        <header class="section-header">
          <div class="section-heading">
            <h2>Conditions</h2>
            <p>Choose how your safety conditions should be evaluated.</p>
          </div>
        </header>

        <div class="section-body">
          <div class="condition-control">
            <div class="condition-copy">
              <span class="control-label">Flag lobby when</span>
              <span class="control-description">
                {{
                  matchModes.find((mode) => mode.value === queueSafety.matchMode)
                    ?.description
                }}
              </span>
            </div>

            <div class="mode-control no-drag">
              <button
                v-for="mode in matchModes"
                :key="mode.value"
                type="button"
                class="mode-option"
                :class="{ 'mode-option--active': queueSafety.matchMode === mode.value }"
                @click="queueSafety.matchMode = mode.value"
              >
                {{ mode.label }}
              </button>
            </div>
          </div>
        </div>
      </section>

      <section class="queue-section">
        <header class="section-header">
          <div class="section-heading">
            <h2>Player Filter</h2>
            <p>Exclude specific players from safety evaluation.</p>
          </div>
        </header>

        <div class="section-body">
          <div class="filter-list">
            <div
              v-for="excludedPlayer in queueSafety.excludedPlayers"
              :key="excludedPlayer.id"
              class="filter-item"
            >
              <div class="filter-item-copy">
                <span class="control-label">Excluded player</span>

                <input
                  v-model.trim="excludedPlayer.username"
                  type="text"
                  maxlength="16"
                  spellcheck="false"
                  placeholder="Enter username"
                  class="input-field filter-input"
                />
              </div>

              <div class="item-action">
                <Button
                  v-if="pendingExcludedIds.has(excludedPlayer.id)"
                  variant="ghost"
                  size="icon-sm"
                  class="action-button action-button--confirm"
                  @click="confirmExcludedPlayer(excludedPlayer.id)"
                >
                  <Check :size="13" />
                </Button>

                <Button
                  v-else
                  variant="ghost"
                  size="icon-sm"
                  class="action-button"
                  @click="removeExcludedPlayer(excludedPlayer.id)"
                >
                  <Trash2 :size="13" />
                </Button>
              </div>
            </div>

            <div
              v-if="queueSafety.excludedPlayers.length === 0"
              class="empty-state"
            >
              <span class="empty-state-title">No players excluded</span>
              <span class="empty-state-description">
                Add a username to ignore that player during evaluation.
              </span>
            </div>
          </div>

          <Button
            variant="outline"
            size="sm"
            class="add-button no-drag"
            @click="addExcludedPlayer"
          >
            <Plus :size="12" />
            Add username
          </Button>
        </div>
      </section>

      <section class="queue-section queue-section--rules">
        <header class="section-header">
          <div class="section-heading">
            <h2>Risk Conditions</h2>
            <p>Define the conditions that should flag a lobby.</p>
          </div>

          <span class="condition-count">
            {{ queueSafety.rules.length }}
            {{ queueSafety.rules.length === 1 ? 'condition' : 'conditions' }}
          </span>
        </header>

        <div class="section-body">
          <div class="rules-list">
            <div
              v-for="rule in queueSafety.rules"
              :key="rule.id"
              class="rule-card"
            >
              <div class="rule-content">
                <template v-if="isUsernameMetric(rule.metric)">
                  <span class="rule-prefix">Flag when</span>

                  <Select
                    class="rule-metric"
                    :model-value="rule.metric"
                    :options="QueueMetricOptions"
                    @update:model-value="
                      (value) => onMetricChange(rule, value as QueueMetric)
                    "
                  />

                  <span class="rule-operator">is exactly</span>

                  <input
                    v-model.trim="rule.username"
                    type="text"
                    maxlength="16"
                    spellcheck="false"
                    placeholder="Username"
                    class="input-field rule-username"
                  />
                </template>

                <template v-else>
                  <span class="rule-prefix">Flag when</span>

                  <div class="player-count-control">
                    <input
                      v-model.number="rule.minPlayers"
                      type="number"
                      min="1"
                      max="16"
                      class="input-field player-count-input"
                      @blur="clampMinPlayers(rule)"
                    />

                    <span> or more player{{ rule.minPlayers === 1 ? '' : 's' }} </span>
                  </div>

                  <Select
                    class="rule-metric"
                    :model-value="rule.metric"
                    :options="QueueMetricOptions"
                    @update:model-value="
                      (value) => onMetricChange(rule, value as QueueMetric)
                    "
                  />

                  <template v-if="!isBooleanMetric(rule.metric)">
                    <Select
                      class="rule-comparator"
                      :model-value="rule.comparator"
                      :options="comparators"
                      @update:model-value="
                        (value) => (rule.comparator = value as ValueComparator)
                      "
                    />

                    <input
                      v-model.number="rule.value"
                      type="number"
                      step="0.1"
                      class="input-field rule-value"
                      @blur="clampValue(rule)"
                    />
                  </template>
                </template>
              </div>

              <div class="rule-action">
                <Button
                  v-if="pendingRuleIds.has(rule.id)"
                  variant="ghost"
                  size="icon-sm"
                  class="action-button action-button--confirm"
                  @click="confirmRule(rule.id)"
                >
                  <Check :size="13" />
                </Button>

                <Button
                  v-else
                  variant="ghost"
                  size="icon-sm"
                  class="action-button"
                  @click="removeRule(rule.id)"
                >
                  <Trash2 :size="13" />
                </Button>
              </div>
            </div>

            <div
              v-if="queueSafety.rules.length === 0"
              class="empty-state"
            >
              <span class="empty-state-title">No risk conditions</span>
              <span class="empty-state-description">
                Add a condition to automatically flag matching lobbies.
              </span>
            </div>
          </div>

          <Button
            variant="outline"
            size="sm"
            class="add-button no-drag"
            @click="addRule"
          >
            <Plus :size="12" />
            Add condition
          </Button>
        </div>
      </section>

      <section class="queue-section">
        <header class="section-header">
          <div class="section-heading">
            <h2>Live Preview</h2>
            <p>See how the current rules evaluate the tracked lobby.</p>
          </div>
        </header>

        <div class="section-body">
          <div class="preview-stat">
            <span>Tracked players</span>
            <strong>{{ players.players.length }}</strong>
          </div>

          <div
            class="preview-verdict"
            :class="{ 'preview-verdict--unsafe': verdict.unsafe }"
          >
            <div class="preview-icon">
              <ShieldAlert
                v-if="verdict.unsafe"
                :size="14"
              />
              <ShieldCheck
                v-else
                :size="14"
              />
            </div>

            <div class="preview-copy">
              <span class="preview-title">
                <template v-if="verdict.unsafe">
                  This lobby matches your safety conditions.
                </template>
                <template v-else> No conditions currently match this lobby. </template>
              </span>

              <div
                v-if="verdict.reasons.length"
                class="preview-reasons"
              >
                <span
                  v-for="(reason, index) in verdict.reasons"
                  :key="index"
                  class="preview-chip"
                >
                  {{ capitalize(reason) }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<style scoped>
.queue-outer {
  position: relative;
  display: flex;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
  border-radius: 0;
}

.queue-banner {
  position: relative;
  flex: 0 0 auto;
  overflow: hidden;
  margin-bottom: 10px;
  background: var(--panel-bg);
  backdrop-filter: var(--panel-blur);
  -webkit-backdrop-filter: var(--panel-blur);
  border-bottom: 1px solid rgba(var(--color-accent-rgb), 0.18);
}

.queue-banner::before {
  content: '';
  position: absolute;
  inset: 0;
  width: 300%;
  left: -100%;
  background: linear-gradient(
    105deg,
    rgba(var(--color-accent-rgb), 0.07) 0%,
    rgba(var(--color-accent-rgb), 0.03) 50%,
    rgba(var(--color-accent-rgb), 0.05) 100%
  );
  will-change: transform;
  animation: banner-shimmer 6s ease-in-out infinite;
  animation-play-state: var(--anim-play-state, running);
  pointer-events: none;
}

.queue-banner-inner {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  min-width: 0;
  padding: 0.7rem 0.85rem;
}

.queue-banner-copy {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 0.65rem;
}

.queue-banner-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  flex: 0 0 32px;
  border-radius: 999px;
}

.queue-banner-text {
  min-width: 0;
}

.queue-banner-title {
  overflow: hidden;
  font-size: 0.85rem;
  font-weight: 600;
  line-height: 1.2;
  color: var(--color-accent-light);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.queue-banner-desc {
  margin-top: 2px;
  overflow: hidden;
  font-size: 0.72rem;
  line-height: 1.35;
  color: var(--color-ink-3);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.queue-content {
  position: relative;
  display: flex;
  flex: 1 1 0;
  width: 100%;
  height: 0;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  gap: 12px;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 0.85rem;
  font-size: 0.92rem;
  scrollbar-gutter: stable;
}

.queue-section {
  position: relative;
  display: flex;
  width: 100%;
  min-width: 0;
  flex: 0 0 auto;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 15px;
  background: rgba(255, 255, 255, 0.018);
  box-shadow:
    0 0 0 1px rgba(0, 0, 0, 0.35),
    0 14px 45px rgba(0, 0, 0, 0.38),
    inset 0 1px rgba(255, 255, 255, 0.035);
}

.queue-section--rules {
  z-index: 3;
  overflow: visible;
}

.queue-section--rules .rule-card {
  position: relative;
  z-index: 1;
}

.queue-section--rules .rule-card:focus-within {
  z-index: 10;
}

.queue-section--rules .rule-card:hover {
  z-index: 5;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
  padding: 13px 14px 11px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  background: rgba(255, 255, 255, 0.01);
}

.section-heading {
  min-width: 0;
}

.section-heading h2 {
  margin: 0;
  font-size: 11px;
  font-weight: 700;
  line-height: 16px;
  letter-spacing: 0.12em;
  color: var(--color-accent);
  text-transform: uppercase;
}

.section-heading p {
  margin: 4px 0 0;
  font-size: 10px;
  line-height: 15px;
  color: var(--color-ink-3);
}

.section-body {
  display: flex;
  width: 100%;
  min-width: 0;
  flex-direction: column;
  gap: 10px;
  padding: 12px 14px 14px;
}

.condition-control {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-width: 0;
  gap: 18px;
  padding: 10px 11px;
  border: 1px solid rgba(255, 255, 255, 0.055);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.018);
}

.condition-copy {
  display: flex;
  min-width: 0;
  flex: 1 1 auto;
  flex-direction: column;
  gap: 3px;
}

.control-label {
  font-size: 11px;
  font-weight: 600;
  line-height: 16px;
  color: var(--color-ink-2);
}

.control-description {
  font-size: 10px;
  line-height: 15px;
  color: var(--color-ink-3);
}

.mode-control {
  display: flex;
  flex: 0 0 auto;
  padding: 2px;
  border: 1px solid rgba(var(--color-accent-rgb), 0.2);
  border-radius: 9px;
  background: rgba(0, 0, 0, 0.12);
}

.mode-option {
  min-width: 56px;
  height: 28px;
  padding: 0 12px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--color-ink-3);
  font-size: 10px;
  font-weight: 600;
  cursor: pointer;
  transition:
    background 140ms ease,
    color 140ms ease;
}

.mode-option:hover {
  color: var(--color-ink-2);
}

.mode-option--active {
  background: rgba(var(--color-accent-rgb), 0.13);
  color: var(--color-accent-light);
}

.filter-list,
.rules-list {
  display: flex;
  width: 100%;
  min-width: 0;
  flex-direction: column;
  gap: 7px;
}

.filter-item,
.rule-card {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
  min-width: 0;
  gap: 10px;
  padding: 9px 10px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.018);
}

.filter-item-copy {
  display: flex;
  align-items: center;
  min-width: 0;
  flex: 1 1 auto;
  gap: 10px;
}

.filter-input {
  width: min(280px, 100%);
  min-width: 0;
  flex: 1 1 180px;
}

.item-action,
.rule-action {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
}

.action-button {
  width: 28px;
  height: 28px;
  color: var(--color-ink-3);
}

.action-button:hover {
  background: rgba(248, 113, 113, 0.1);
  color: var(--color-bad);
}

.action-button--confirm {
  color: var(--color-accent-light);
}

.action-button--confirm:hover {
  background: rgba(var(--color-accent-rgb), 0.12);
  color: var(--color-accent-light);
}

.empty-state {
  display: flex;
  width: 100%;
  min-height: 58px;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 2px;
  padding: 10px 14px;
  border: 1px dashed rgba(255, 255, 255, 0.07);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.06);
  text-align: center;
}

.empty-state-title {
  font-size: 10px;
  font-weight: 600;
  line-height: 15px;
  color: var(--color-ink-2);
}

.empty-state-description {
  max-width: 420px;
  font-size: 10px;
  line-height: 15px;
  color: var(--color-ink-3);
}

.add-button {
  align-self: flex-start;
  height: 29px;
  gap: 6px;
}

.condition-count {
  flex: 0 0 auto;
  padding: 3px 7px;
  border: 1px solid rgba(var(--color-accent-rgb), 0.18);
  border-radius: 999px;
  background: rgba(var(--color-accent-rgb), 0.05);
  color: var(--color-ink-3);
  font-size: 9px;
  font-weight: 600;
}

.rule-content {
  display: flex;
  align-items: center;
  min-width: 0;
  flex: 1 1 auto;
  flex-wrap: wrap;
  gap: 7px;
}

.rule-prefix,
.rule-operator {
  flex: 0 0 auto;
  font-size: 10px;
  font-weight: 550;
  line-height: 30px;
  color: var(--color-ink-2);
  white-space: nowrap;
}

.rule-operator {
  color: var(--color-ink-3);
}

.rule-metric {
  width: 145px;
  max-width: 100%;
  flex: 0 1 145px;
}

.rule-comparator {
  width: 108px;
  max-width: 100%;
  flex: 0 1 108px;
}

.rule-username {
  width: 160px;
  min-width: 100px;
  flex: 1 1 160px;
}

.player-count-control {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 6px;
  color: var(--color-ink-3);
  font-size: 10px;
  line-height: 30px;
  white-space: nowrap;
}

.player-count-input {
  width: 54px;
  height: 30px;
  flex: 0 0 54px;
  text-align: center;
}

.rule-value {
  width: 76px;
  height: 30px;
  flex: 0 1 76px;
}

.preview-stat {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 30px;
  padding: 0 2px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.045);
}

.preview-stat span {
  font-size: 10px;
  color: var(--color-ink-3);
}

.preview-stat strong {
  font-size: 11px;
  font-weight: 650;
  color: var(--color-ink-2);
}

.preview-verdict {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 9px;
  padding: 9px 10px;
  border: 1px solid rgba(255, 255, 255, 0.065);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.018);
}

.preview-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  flex: 0 0 28px;
  border: 1px solid rgba(var(--color-accent-rgb), 0.25);
  border-radius: 50%;
  background: rgba(var(--color-accent-rgb), 0.1);
  color: var(--color-accent-light);
}

.preview-copy {
  display: flex;
  min-width: 0;
  flex: 1 1 auto;
  flex-direction: column;
  gap: 5px;
}

.preview-title {
  overflow: hidden;
  font-size: 10px;
  font-weight: 600;
  line-height: 15px;
  color: var(--color-ink-2);
  text-overflow: ellipsis;
}

.preview-verdict--unsafe {
  border-color: rgba(248, 113, 113, 0.25);
  background: rgba(248, 113, 113, 0.06);
}

.preview-verdict--unsafe .preview-icon {
  border-color: rgba(248, 113, 113, 0.32);
  background: rgba(248, 113, 113, 0.12);
  color: var(--color-bad);
}

.preview-verdict--unsafe .preview-title {
  color: var(--color-bad);
}

.preview-reasons {
  display: flex;
  min-width: 0;
  flex-wrap: wrap;
  gap: 4px;
}

.preview-chip {
  max-width: 100%;
  overflow: hidden;
  padding: 2px 7px;
  border: 1px solid rgba(255, 255, 255, 0.07);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.04);
  color: var(--color-ink-3);
  font-size: 9px;
  line-height: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-verdict--unsafe .preview-chip {
  border-color: rgba(248, 113, 113, 0.18);
  background: rgba(248, 113, 113, 0.07);
  color: rgba(248, 113, 113, 0.9);
}

@media (min-width: 640px) {
  .queue-outer {
    border-radius: 12px 12px 0 0;
  }
}

@media (max-width: 760px) {
  .condition-control {
    align-items: stretch;
    flex-direction: column;
    gap: 10px;
  }

  .mode-control {
    width: 100%;
  }

  .mode-option {
    flex: 1 1 0;
  }

  .filter-item-copy {
    align-items: stretch;
    flex-direction: column;
    gap: 6px;
  }

  .filter-input {
    width: 100%;
    flex-basis: auto;
  }

  .rule-card {
    align-items: stretch;
  }

  .rule-content {
    align-items: stretch;
    flex-direction: column;
  }

  .rule-prefix,
  .rule-operator {
    line-height: 18px;
  }

  .rule-metric,
  .rule-comparator,
  .rule-username,
  .rule-value {
    width: 100%;
    flex: 1 1 auto;
  }

  .player-count-control {
    width: 100%;
  }

  .player-count-input {
    flex: 0 0 54px;
  }

  .rule-action {
    align-self: flex-start;
  }
}

@media (max-width: 480px) {
  .queue-content {
    gap: 9px;
    padding: 0.65rem;
  }

  .queue-banner-inner {
    padding: 0.65rem 0.7rem;
  }

  .queue-banner-desc {
    max-width: 220px;
  }

  .section-header {
    padding: 11px 11px 9px;
  }

  .section-body {
    padding: 10px 11px 11px;
  }

  .condition-control {
    padding: 9px;
  }

  .filter-item,
  .rule-card {
    padding: 8px;
  }
}

@media (max-width: 360px) {
  .queue-banner-icon {
    width: 29px;
    height: 29px;
    flex-basis: 29px;
  }

  .queue-banner-title {
    font-size: 0.8rem;
  }

  .queue-banner-desc {
    font-size: 0.67rem;
  }

  .queue-content {
    padding: 0.5rem;
  }

  .rule-prefix,
  .rule-operator,
  .control-label,
  .row-label {
    font-size: 0.73rem;
  }

  .control-description,
  .row-desc {
    font-size: 0.66rem;
  }
}

@keyframes banner-shimmer {
  0% {
    transform: translateX(-8%);
  }

  50% {
    transform: translateX(8%);
  }

  100% {
    transform: translateX(-8%);
  }
}
</style>
