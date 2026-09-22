<script setup lang="ts">
import type { SupportMessage } from '@renderer/types';
import { Button } from '@renderer/components/ui/button';
import { useSupport } from '@renderer/composables/useSupport';
import { useTelemetryAuth } from '@renderer/composables/useTelemetryAuth';
import { LifeBuoy, RefreshCw, Send, X } from 'lucide-vue-next';
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';

const {
  checked: authChecked,
  linked: discordLinked,
  linking: discordLinking,
  refreshStatus,
  startLink,
} = useTelemetryAuth();

const {
  conversations,
  conversationsError,
  activeConversation,
  activeConversationLoading,
  createError,
  replyError,
  socketConnected,
  refreshConversations,
  refreshActiveConversation,
  openConversation,
  createTicket,
  sendReply,
  startRealtime,
  stopRealtime,
} = useSupport();

const threadRoot = ref<HTMLElement | null>(null);
const composerField = ref<HTMLTextAreaElement | null>(null);
const replyText = ref('');
const initializing = ref(false);

function formatTime(value: string | number): string {
  const date = typeof value === 'number' ? new Date(value) : new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
}

function formatDayLabel(value: string | number): string {
  const date = typeof value === 'number' ? new Date(value) : new Date(value);
  if (Number.isNaN(date.getTime())) return '';

  const now = new Date();
  const isSameDay = (a: Date, b: Date): boolean =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate();

  if (isSameDay(date, now)) return 'Today';

  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (isSameDay(date, yesterday)) return 'Yesterday';

  return date.toLocaleDateString('en-US', { month: 'long', day: 'numeric' });
}

function deriveSubject(text: string): string {
  const normalized = text.replace(/\s+/g, ' ').trim();
  return normalized.length > 60 ? `${normalized.slice(0, 57)}…` : normalized;
}

interface DisplayMessage extends SupportMessage {
  senderLabel: string;
  showMeta: boolean;
  showTail: boolean;
  dayLabel: string | null;
}

const displayMessages = computed<DisplayMessage[]>(() => {
  const messages = activeConversation.value?.messages ?? [];
  const result: DisplayMessage[] = [];

  messages.forEach((message, index) => {
    const previous = messages[index - 1];
    const next = messages[index + 1];

    const sameSenderAsPrev =
      previous !== undefined &&
      previous.fromSupport === message.fromSupport &&
      (previous.senderName ?? null) === (message.senderName ?? null);
    const sameSenderAsNext =
      next !== undefined &&
      next.fromSupport === message.fromSupport &&
      (next.senderName ?? null) === (message.senderName ?? null);

    const withinGroupWindow = (a: string, b: string): boolean => {
      const diff = Math.abs(new Date(b).getTime() - new Date(a).getTime());
      return Number.isFinite(diff) && diff < 5 * 60 * 1000;
    };

    const groupedWithPrev =
      sameSenderAsPrev && withinGroupWindow(previous.createdAt, message.createdAt);
    const groupedWithNext =
      sameSenderAsNext && withinGroupWindow(message.createdAt, next.createdAt);

    const dayLabel =
      previous === undefined ||
      formatDayLabel(previous.createdAt) !== formatDayLabel(message.createdAt)
        ? formatDayLabel(message.createdAt)
        : null;

    result.push({
      ...message,
      senderLabel: message.fromSupport ? (message.senderName ?? 'Support') : 'You',
      showMeta: !groupedWithPrev || dayLabel !== null,
      showTail: !groupedWithNext,
      dayLabel,
    });
  });

  return result;
});

const avatarErrored = ref(new Set<string>());

function onAvatarError(messageId: string): void {
  avatarErrored.value = new Set(avatarErrored.value).add(messageId);
}

const composing = computed(() => authChecked.value && discordLinked.value);
const composeError = computed(() =>
  activeConversation.value ? replyError.value : createError.value,
);

interface PendingMessage {
  localId: string;
  body: string;
  status: 'sending' | 'failed';
}

const pendingMessages = ref<PendingMessage[]>([]);
let pendingCounter = 0;

function nextPendingId(): string {
  pendingCounter += 1;
  return `pending-${Date.now()}-${pendingCounter}`;
}

const hasThreadContent = computed(
  () => !!activeConversation.value || pendingMessages.value.length > 0,
);

async function scrollThreadToBottom(smooth = true): Promise<void> {
  await nextTick();
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  const el = threadRoot.value;
  if (!el) return;
  el.scrollTo({ top: el.scrollHeight, behavior: smooth ? 'smooth' : 'auto' });
}

async function openMostRecentConversation(): Promise<void> {
  if (activeConversation.value || !conversations.value.length) return;

  const [latest] = [...conversations.value].sort((a, b) => {
    const at = new Date(a.updatedAt ?? a.createdAt).getTime();
    const bt = new Date(b.updatedAt ?? b.createdAt).getTime();
    return bt - at;
  });

  await openConversation(latest.id);
  await scrollThreadToBottom(false);
}

async function dispatchMessage(localId: string, body: string): Promise<void> {
  const ok = activeConversation.value
    ? await sendReply(body)
    : await createTicket(deriveSubject(body), body);

  if (ok) {
    pendingMessages.value = pendingMessages.value.filter((m) => m.localId !== localId);
  } else {
    const target = pendingMessages.value.find((m) => m.localId === localId);
    if (target) target.status = 'failed';
  }
}

async function submitReply(): Promise<void> {
  const text = replyText.value.trim();
  if (!text) return;

  replyText.value = '';
  resetComposerHeight();

  const localId = nextPendingId();
  pendingMessages.value = [
    ...pendingMessages.value,
    { localId, body: text, status: 'sending' },
  ];
  await scrollThreadToBottom();

  await dispatchMessage(localId, text);
}

async function retryPending(localId: string): Promise<void> {
  const target = pendingMessages.value.find((m) => m.localId === localId);
  if (!target) return;
  target.status = 'sending';
  await dispatchMessage(localId, target.body);
}

function dismissPending(localId: string): void {
  pendingMessages.value = pendingMessages.value.filter((m) => m.localId !== localId);
}

function autoGrowComposer(): void {
  const el = composerField.value;
  if (!el) return;
  el.style.height = 'auto';
  el.style.height = `${Math.min(el.scrollHeight, 88)}px`;
}

function resetComposerHeight(): void {
  const el = composerField.value;
  if (!el) return;
  el.style.height = 'auto';
}

async function manualRefresh(): Promise<void> {
  if (activeConversation.value) {
    await refreshActiveConversation();
  } else {
    await refreshConversations();
    await openMostRecentConversation();
  }
  await scrollThreadToBottom();
}

watch(
  () => activeConversation.value?.messages.length,
  () => {
    void scrollThreadToBottom();
  },
);

watch(
  () => pendingMessages.value.length,
  () => {
    void scrollThreadToBottom();
  },
);

watch(discordLinked, async (linked) => {
  if (!linked) return;
  initializing.value = true;
  await refreshConversations();
  await openMostRecentConversation();
  startRealtime();
  initializing.value = false;
});

onMounted(async () => {
  await refreshStatus();
  if (discordLinked.value) {
    initializing.value = true;
    await refreshConversations();
    await openMostRecentConversation();
    startRealtime();
    initializing.value = false;
  }
});

onUnmounted(() => {
  stopRealtime();
});
</script>

<template>
  <div class="animate-fade-in flex h-full flex-col overflow-hidden">
    <div
      class="no-drag support-banner flex shrink-0 items-center justify-between gap-3 px-3.5 py-2.5"
    >
      <div class="flex min-w-0 items-center gap-2.5">
        <div
          class="glow-halo support-icon flex h-8 w-8 shrink-0 items-center justify-center rounded-full"
        >
          <LifeBuoy
            :size="15"
            style="color: var(--color-accent-light)"
          />
        </div>
        <div class="min-w-0">
          <div
            class="flex min-w-0 items-center gap-1.5"
            style="
              font-size: 0.85rem;
              font-weight: 600;
              color: var(--color-accent-light);
              line-height: 1.2;
              letter-spacing: 0.01em;
            "
          >
            <span class="truncate">Support</span>
          </div>
          <div
            class="flex items-center gap-1.5"
            style="
              font-size: 0.76rem;
              color: var(--color-ink-3);
              line-height: 1.35;
              margin-top: 1px;
            "
          >
            <span class="status-dot-wrap">
              <span
                class="status-dot"
                :style="{
                  backgroundColor: socketConnected
                    ? 'var(--color-good)'
                    : 'var(--color-ink-3)',
                  boxShadow: socketConnected
                    ? '0 0 6px rgba(52, 211, 153, 0.65)'
                    : 'none',
                  transition: 'background-color 200ms ease, box-shadow 200ms ease',
                }"
              />
            </span>
            <span class="truncate">{{
              socketConnected ? 'Connected' : 'Reconnecting…'
            }}</span>
          </div>
        </div>
      </div>

      <Button
        v-if="discordLinked"
        variant="ghost"
        size="icon-sm"
        :disabled="initializing || activeConversationLoading"
        title="Refresh"
        @click="manualRefresh"
      >
        <RefreshCw
          :size="12.5"
          :class="{ 'animate-spin': initializing || activeConversationLoading }"
        />
      </Button>
    </div>

    <div class="flex flex-1 flex-col overflow-hidden">
      <div
        v-if="!authChecked"
        class="flex h-full flex-col items-center justify-center gap-2.5"
        style="opacity: 0.5"
      >
        <p style="font-size: 0.78rem; color: var(--color-ink-3)">
          Checking your account…
        </p>
      </div>

      <div
        v-else-if="!discordLinked"
        class="flex h-full flex-col items-center justify-center gap-3 px-6 text-center"
      >
        <div
          class="glow-halo support-icon flex h-11 w-11 items-center justify-center rounded-full"
        >
          <LifeBuoy
            :size="19"
            style="color: var(--color-accent-light)"
          />
        </div>
        <p style="font-size: 0.83rem; color: var(--color-ink-1); font-weight: 600">
          Link your Discord account to use Support
        </p>
        <p
          style="
            font-size: 0.78rem;
            color: var(--color-ink-3);
            max-width: 260px;
            line-height: 1.5;
          "
        >
          Chats are tied to your Discord identity so our team knows who they're replying
          to.
        </p>
        <Button
          variant="control"
          size="sm"
          :disabled="discordLinking"
          @click="startLink"
        >
          {{ discordLinking ? 'Linking…' : 'Link Discord' }}
        </Button>
      </div>

      <template v-else>
        <div
          v-if="conversationsError"
          class="mx-3.5 mt-3 rounded-md px-3 py-2"
          style="
            background: rgba(248, 113, 113, 0.08);
            border: 1px solid rgba(248, 113, 113, 0.24);
            font-size: 0.78rem;
            color: #f87171;
          "
        >
          {{ conversationsError }}
        </div>

        <div
          v-if="initializing && !hasThreadContent"
          class="flex h-full flex-col items-center justify-center gap-2.5"
          style="opacity: 0.5"
        >
          <p style="font-size: 0.78rem; color: var(--color-ink-3)">
            Loading conversation…
          </p>
        </div>

        <div
          v-else-if="!hasThreadContent"
          class="flex h-full flex-col items-center justify-center gap-2.5 px-6 text-center"
        >
          <div
            class="glow-halo support-icon flex h-11 w-11 items-center justify-center rounded-full"
            style="opacity: 0.7"
          >
            <LifeBuoy
              :size="19"
              style="color: var(--color-accent-light)"
            />
          </div>
          <p style="font-size: 0.82rem; color: var(--color-ink-1); font-weight: 600">
            Start a conversation
          </p>
          <p
            style="
              font-size: 0.78rem;
              color: var(--color-ink-3);
              max-width: 240px;
              line-height: 1.5;
            "
          >
            Send a message below and our team will pick it up shortly.
          </p>
        </div>

        <div
          v-else
          ref="threadRoot"
          class="themed-scroll flex flex-1 flex-col justify-end overflow-y-auto px-3.5 py-3"
        >
          <template
            v-for="message in displayMessages"
            :key="message.id"
          >
            <div
              v-if="message.dayLabel"
              class="day-divider"
            >
              <span>{{ message.dayLabel }}</span>
            </div>

            <div
              class="flex items-end gap-2"
              :style="{
                justifyContent: message.fromSupport ? 'flex-start' : 'flex-end',
                marginTop: message.showMeta ? '14px' : '2px',
              }"
            >
              <div
                v-if="message.fromSupport"
                class="flex h-6 w-6 shrink-0 items-center justify-center overflow-hidden rounded-full"
                :style="{
                  visibility: message.showTail ? 'visible' : 'hidden',
                  background: 'rgba(var(--color-accent-rgb), 0.14)',
                  border: '1px solid rgba(var(--color-accent-rgb), 0.28)',
                }"
              >
                <img
                  v-if="message.senderAvatarUrl && !avatarErrored.has(message.id)"
                  :src="message.senderAvatarUrl"
                  :alt="message.senderLabel"
                  class="h-full w-full object-cover"
                  referrerpolicy="no-referrer"
                  @error="onAvatarError(message.id)"
                />
                <LifeBuoy
                  v-else
                  :size="11"
                  style="color: var(--color-accent-light)"
                />
              </div>

              <div
                class="flex max-w-[76%] min-w-0 flex-col gap-0.5"
                :style="{ alignItems: message.fromSupport ? 'flex-start' : 'flex-end' }"
              >
                <span
                  v-if="message.showMeta"
                  class="chat-meta"
                >
                  {{ message.senderLabel }} · {{ formatTime(message.createdAt) }}
                </span>
                <div
                  class="chat-bubble"
                  :class="[
                    message.fromSupport ? 'chat-bubble-support' : 'chat-bubble-user',
                    message.showTail
                      ? message.fromSupport
                        ? 'chat-bubble-tail-left'
                        : 'chat-bubble-tail-right'
                      : '',
                  ]"
                >
                  {{ message.body }}
                </div>
              </div>
            </div>
          </template>

          <div
            v-for="pending in pendingMessages"
            :key="pending.localId"
            class="flex items-end justify-end gap-2"
            style="margin-top: 2px"
          >
            <div class="flex max-w-[76%] min-w-0 flex-col items-end gap-0.5">
              <div
                class="chat-bubble chat-bubble-user chat-bubble-tail-right"
                :class="{ 'chat-bubble-sending': pending.status === 'sending' }"
              >
                {{ pending.body }}
              </div>
              <div
                v-if="pending.status === 'failed'"
                class="chat-pending-failed"
              >
                <button
                  type="button"
                  class="chat-pending-failed-retry"
                  @click="retryPending(pending.localId)"
                >
                  <X :size="10" />
                  <span>Failed to send · Tap to retry</span>
                </button>
                <button
                  type="button"
                  class="chat-pending-dismiss"
                  title="Discard"
                  @click="dismissPending(pending.localId)"
                >
                  <X :size="10" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </template>
    </div>

    <div
      v-if="composing"
      class="no-drag composer-bar shrink-0 px-3.5 py-2.5"
    >
      <div
        v-if="composeError"
        class="mb-2 rounded-md px-3 py-1.5"
        style="
          background: rgba(248, 113, 113, 0.08);
          border: 1px solid rgba(248, 113, 113, 0.24);
          font-size: 0.75rem;
          color: #f87171;
        "
      >
        {{ composeError }}
      </div>
      <div class="composer-field flex items-center gap-2">
        <textarea
          ref="composerField"
          v-model="replyText"
          rows="1"
          placeholder="Write a message…"
          class="composer-textarea"
          @input="autoGrowComposer"
          @keydown.enter.exact.prevent="submitReply"
        />
        <Button
          variant="control-icon"
          size="icon"
          class="composer-send-btn"
          :disabled="!replyText.trim()"
          @click="submitReply"
        >
          <Send :size="13" />
        </Button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.support-banner {
  position: relative;
  overflow: hidden;
  border-bottom: 1px solid rgba(var(--color-accent-rgb), 0.18);
}
.support-banner::before {
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

.support-icon {
  background: rgba(var(--color-accent-rgb), 0.12);
  border: 1px solid rgba(var(--color-accent-rgb), 0.28);
}

.status-dot-wrap {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  width: 12px;
  height: 12px;
}

.status-dot {
  display: block;
  width: 6px;
  height: 6px;
  border-radius: 9999px;
}

.day-divider {
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 16px 0 10px;
}
.day-divider span {
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  color: var(--color-ink-3);
  background: var(--color-surface-1);
  border: 1px solid var(--color-border);
  border-radius: 999px;
  padding: 3px 10px;
}

.chat-meta {
  font-size: 0.68rem;
  color: var(--color-ink-3);
  padding: 0 6px;
}

.chat-bubble {
  border-radius: 14px;
  padding: 7px 11px;
  font-size: 0.8rem;
  line-height: 1.48;
  white-space: pre-wrap;
  word-break: break-word;
  box-shadow: var(--shadow-card);
}

.chat-bubble-support {
  background: var(--color-surface-2);
  border: 1px solid var(--color-border);
  color: var(--color-ink-1);
}
.chat-bubble-support.chat-bubble-tail-left {
  border-bottom-left-radius: 4px;
}

.chat-bubble-user {
  background: linear-gradient(
    135deg,
    rgba(var(--color-accent-rgb), 0.28) 0%,
    rgba(var(--color-accent-rgb), 0.16) 100%
  );
  border: 1px solid rgba(var(--color-accent-rgb), 0.36);
  color: var(--color-ink-1);
}
.chat-bubble-user.chat-bubble-tail-right {
  border-bottom-right-radius: 4px;
}

.chat-bubble-sending {
  animation: chat-bubble-sending-pulse 1.2s ease-in-out infinite;
}

@keyframes chat-bubble-sending-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.55;
  }
}

.chat-pending-failed {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 6px;
}

.chat-pending-failed-retry {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: none;
  background: transparent;
  padding: 0;
  cursor: pointer;
  font-size: 0.68rem;
  color: #f87171;
}
.chat-pending-failed-retry:hover {
  text-decoration: underline;
}

.chat-pending-dismiss {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  padding: 0;
  cursor: pointer;
  color: var(--color-ink-3);
}
.chat-pending-dismiss:hover {
  color: var(--color-ink-1);
}

.composer-bar {
  border-top: 1px solid var(--color-border);
  background: linear-gradient(
    180deg,
    transparent 0%,
    rgba(var(--color-accent-rgb), 0.02) 100%
  );
}

.composer-field {
  border-radius: var(--radius-lg);
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid var(--color-border);
  padding: 6px 6px 6px 12px;
  transition:
    border-color 140ms ease,
    background 140ms ease,
    box-shadow 140ms ease;
}
.composer-field:focus-within {
  border-color: rgba(var(--color-accent-rgb), 0.5);
  background: rgba(var(--color-accent-rgb), 0.05);
  box-shadow: 0 0 0 2.5px rgba(var(--color-accent-rgb), 0.14);
}

.composer-textarea {
  flex: 1;
  resize: none;
  border: none;
  outline: none;
  background: transparent;
  color: var(--color-ink-1);
  font-family: var(--font-sans);
  font-size: 0.82rem;
  line-height: 1.4;
  padding: 5px 0;
  max-height: 88px;
}
.composer-textarea::placeholder {
  color: var(--color-ink-3);
}

.composer-send-btn {
  border: none;
  background: transparent;
}
.composer-send-btn:hover:not(:disabled) {
  border: none;
  background: rgba(var(--color-accent-rgb), 0.14);
}
.composer-send-btn:disabled {
  border: none;
  background: transparent;
}
</style>
