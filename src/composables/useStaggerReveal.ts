import type { Ref } from 'vue';
import { duration, ease, gsap, motionEnabled } from '@renderer/lib/motion';
import { useConfigStore } from '@renderer/store/config';
import { onMounted, onUnmounted, watch } from 'vue';

interface StaggerOptions {
  selector: string;
  y?: number;
  stagger?: number;
  watchSource?: () => unknown;
}

export function useStaggerReveal(
  root: Ref<HTMLElement | null>,
  options: StaggerOptions,
): void {
  const config = useConfigStore();
  let ctx: gsap.Context | null = null;

  function play(): void {
    if (!root.value || !motionEnabled(config.lowEndMode)) return;

    ctx?.revert();
    ctx = gsap.context(() => {
      gsap.from(options.selector, {
        opacity: 0,
        y: options.y ?? 8,
        duration: duration.base,
        stagger: options.stagger ?? 0.035,
        ease: ease.out,
      });
    }, root.value);
  }

  onMounted(() => {
    requestAnimationFrame(play);
  });

  if (options.watchSource) {
    watch(options.watchSource, () => {
      requestAnimationFrame(play);
    });
  }

  onUnmounted(() => ctx?.revert());
}
