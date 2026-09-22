import type {
  SupportConversationDetail,
  SupportConversationSummary,
  SupportMessage,
  SupportSocketEvent,
} from '@renderer/types';
import { ref } from 'vue';

const conversations = ref<SupportConversationSummary[]>([]);
const conversationsLoading = ref(false);
const conversationsError = ref('');

const activeConversation = ref<SupportConversationDetail | null>(null);
const activeConversationLoading = ref(false);
const activeConversationError = ref('');
let activeRequestToken = 0;

const creating = ref(false);
const createError = ref('');

const replying = ref(false);
const replyError = ref('');

const socketConnected = ref(false);
let unsubscribeSocket: (() => void) | null = null;

async function refreshConversations(): Promise<void> {
  conversationsLoading.value = true;
  conversationsError.value = '';

  const result = await window.api.support.list();
  if (result.ok) {
    conversations.value = result.data.conversations;
  } else {
    conversationsError.value = result.error;
  }

  conversationsLoading.value = false;
}

async function openConversation(id: string): Promise<void> {
  const token = ++activeRequestToken;
  activeConversationLoading.value = true;
  activeConversationError.value = '';

  const result = await window.api.support.get(id);
  if (token !== activeRequestToken) return;

  if (result.ok) {
    activeConversation.value = result.data;
  } else {
    activeConversationError.value = result.error;
  }

  activeConversationLoading.value = false;
}

async function refreshActiveConversation(): Promise<void> {
  if (!activeConversation.value) return;

  const token = activeRequestToken;
  const result = await window.api.support.get(activeConversation.value.id);
  if (token !== activeRequestToken) return;

  if (result.ok) {
    activeConversation.value = result.data;
  }
}

function closeConversation(): void {
  activeRequestToken += 1;
  activeConversation.value = null;
  activeConversationError.value = '';
}

async function createTicket(subject: string, message: string): Promise<boolean> {
  creating.value = true;
  createError.value = '';

  const result = await window.api.support.create(subject, message);
  creating.value = false;

  if (!result.ok) {
    createError.value = result.error;
    return false;
  }

  await refreshConversations();
  await openConversation(result.data.id);
  return true;
}

async function sendReply(message: string): Promise<boolean> {
  if (!activeConversation.value) return false;

  replying.value = true;
  replyError.value = '';

  const result = await window.api.support.reply(activeConversation.value.id, message);
  replying.value = false;

  if (!result.ok) {
    replyError.value = result.error;
    return false;
  }

  await refreshActiveConversation();
  return true;
}

function upsertActiveMessage(message: SupportMessage): void {
  const current = activeConversation.value;
  if (!current) return;

  const existingIndex = current.messages.findIndex((m) => m.id === message.id);
  const messages =
    existingIndex === -1
      ? [...current.messages, message]
      : current.messages.map((m, i) => (i === existingIndex ? message : m));

  activeConversation.value = { ...current, messages };
}

function handleSocketEvent(event: SupportSocketEvent): void {
  if (event.type === 'connected') {
    socketConnected.value = true;
    void refreshConversations();
    void refreshActiveConversation();
    return;
  }

  if (event.type === 'disconnected') {
    socketConnected.value = false;
    return;
  }

  if (activeConversation.value?.id === event.conversationId) {
    upsertActiveMessage(event.message);
  }
  void refreshConversations();
}

function startRealtime(): void {
  if (!unsubscribeSocket) {
    unsubscribeSocket = window.api.support.onSocketEvent(handleSocketEvent);
  }
  window.api.support.connectSocket();
}

function stopRealtime(): void {
  window.api.support.disconnectSocket();
  unsubscribeSocket?.();
  unsubscribeSocket = null;
  socketConnected.value = false;
}

export function useSupport() {
  return {
    conversations,
    conversationsLoading,
    conversationsError,
    activeConversation,
    activeConversationLoading,
    activeConversationError,
    creating,
    createError,
    replying,
    replyError,
    socketConnected,
    refreshConversations,
    openConversation,
    refreshActiveConversation,
    closeConversation,
    createTicket,
    sendReply,
    startRealtime,
    stopRealtime,
  };
}
