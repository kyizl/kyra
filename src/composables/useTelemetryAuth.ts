import type { TelemetryEventPayload } from '@renderer/types';
import { computed, ref } from 'vue';

const checked = ref(false);
const linked = ref(false);
const linking = ref(false);
const errorMessage = ref('');
const disclosureAcknowledged = ref(true);
const acknowledging = ref(false);
const authorizeUrl = ref('');

const needsLink = computed(() => checked.value && !linked.value);
const checking = computed(() => !checked.value);
const needsDisclosure = computed(
  () => checked.value && linked.value && !disclosureAcknowledged.value,
);

async function refreshStatus(): Promise<void> {
  checked.value = false;
  errorMessage.value = '';
  authorizeUrl.value = '';

  try {
    linked.value = await window.api.telemetry.isLinked();
    disclosureAcknowledged.value = linked.value
      ? await window.api.telemetry.isDisclosureAcknowledged()
      : true;
  } catch {
    linked.value = false;
    errorMessage.value =
      'Unable to verify Discord login. Please make sure you are connected to the internet.';
  } finally {
    checked.value = true;
  }
}

function startLink(): void {
  errorMessage.value = '';
  authorizeUrl.value = '';
  linking.value = true;
  window.api.telemetry.startLink();
}

function bypassLink(): void {
  errorMessage.value = '';
  linking.value = false;
  linked.value = true;
  disclosureAcknowledged.value = true;
  checked.value = true;
}

async function acknowledgeDisclosure(): Promise<void> {
  if (acknowledging.value) return;

  acknowledging.value = true;
  try {
    await window.api.telemetry.acknowledgeDisclosure();
    disclosureAcknowledged.value = true;
  } finally {
    acknowledging.value = false;
  }
}

function handleTelemetryEvent(event: TelemetryEventPayload): void {
  if (event.type === 'linking') {
    linking.value = true;
    errorMessage.value = '';
    authorizeUrl.value = event.authorizeUrl;
    return;
  }

  if (event.type === 'linked') {
    linking.value = false;
    linked.value = true;
    disclosureAcknowledged.value = true;
    return;
  }

  linking.value = false;
  errorMessage.value = event.message;
  authorizeUrl.value = '';
}

export function useTelemetryAuth() {
  return {
    checked,
    linked,
    linking,
    checking,
    errorMessage,
    authorizeUrl,
    needsLink,
    needsDisclosure,
    acknowledging,
    refreshStatus,
    acknowledgeDisclosure,
    startLink,
    bypassLink,
    handleTelemetryEvent,
  };
}
