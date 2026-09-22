import type { PiniaPluginContext } from 'pinia';
import { perfEnabled, recordStore } from '@renderer/lib/perf-bus';

export function perfTimingPlugin({ store }: PiniaPluginContext): void {
  store.$onAction(({ name, after, onError }) => {
    if (!perfEnabled.value) return;

    const start = performance.now();

    after(() => {
      recordStore(`${store.$id}.${name}`, performance.now() - start);
    });

    onError(() => {
      recordStore(`${store.$id}.${name}`, performance.now() - start, { error: true });
    });
  });
}
