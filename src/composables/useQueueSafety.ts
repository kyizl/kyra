import type { RuleMatch } from '@renderer/types/queue-safety';
import { useConfigStore } from '@renderer/store/config';
import { usePlayersStore } from '@renderer/store/players';
import { evaluateQueueSafety } from '@renderer/types/queue-safety';
import { computed, ref, watch } from 'vue';

function matchSignature(triggeredRules: RuleMatch[]): string {
  return triggeredRules
    .map(
      (match) =>
        `${match.rule.id}:${match.matchedPlayers
          .map((player) => player.uuid ?? player.name)
          .sort()
          .join(',')}`,
    )
    .sort()
    .join('|');
}

export function useQueueSafety() {
  const config = useConfigStore();
  const players = usePlayersStore();

  const verdict = computed(() =>
    evaluateQueueSafety(config.queueSafety, players.players),
  );

  const dismissed = ref(false);
  const lastSignature = ref('');

  watch(
    verdict,
    (current) => {
      if (!current.unsafe) {
        dismissed.value = false;
        lastSignature.value = '';
        return;
      }

      const signature = matchSignature(current.triggeredRules);
      if (signature !== lastSignature.value) {
        dismissed.value = false;
        lastSignature.value = signature;
      }
    },
    { immediate: true },
  );

  function dismiss(): void {
    dismissed.value = true;
  }

  const visible = computed(() => verdict.value.unsafe && !dismissed.value);

  return { verdict, visible, dismiss };
}
