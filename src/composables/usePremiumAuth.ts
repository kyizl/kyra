import type { ProxyEventPayload } from '@renderer/types';
import { computed, ref } from 'vue';

export interface PremiumAuthState {
  network: 'pikanetwork' | 'jartexnetwork';
  userCode: string;
  verificationUri: string;
  expiresAt: number;
  status: 'waiting' | 'success' | 'error';
  errorMessage: string;
}

const state = ref<PremiumAuthState | null>(null);

const active = computed<PremiumAuthState | null>(() => state.value);

function handleProxyAuthEvent(event: ProxyEventPayload): void {
  if (event.type === 'auth-code') {
    state.value = {
      network: event.network as 'pikanetwork' | 'jartexnetwork',
      userCode: event.userCode,
      verificationUri: event.verificationUri,
      expiresAt: Date.now() + event.expiresInSeconds * 1000,
      status: 'waiting',
      errorMessage: '',
    };

    return;
  }

  if (event.type === 'auth-success') {
    if (state.value && state.value.network === event.network) {
      state.value = { ...state.value, status: 'success' };

      setTimeout(() => {
        if (state.value?.status === 'success') {
          state.value = null;
        }
      }, 2200);
    }

    return;
  }

  if (event.type === 'auth-error') {
    if (state.value && state.value.network === event.network) {
      state.value = {
        ...state.value,
        status: 'error',
        errorMessage: event.message,
      };
    }
  }
}

function dismiss(): void {
  state.value = null;
}

export function usePremiumAuth() {
  return {
    active,
    handleProxyAuthEvent,
    dismiss,
  };
}
