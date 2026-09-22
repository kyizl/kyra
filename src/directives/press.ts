import type { Directive } from 'vue';
import { gsap, prefersReducedMotion } from '@renderer/lib/motion';

interface PressState {
  cleanup: () => void;
}

const registry = new WeakMap<HTMLElement, PressState>();

export const vPress: Directive<HTMLElement, number | void> = {
  mounted(el, binding) {
    if (prefersReducedMotion()) return;

    const magnetStrength = typeof binding.value === 'number' ? binding.value : 0;

    const scaleTo = gsap.quickTo(el, 'scale', { duration: 0.22, ease: 'power2.out' });
    const xTo = magnetStrength
      ? gsap.quickTo(el, 'x', { duration: 0.35, ease: 'power3' })
      : null;
    const yTo = magnetStrength
      ? gsap.quickTo(el, 'y', { duration: 0.35, ease: 'power3' })
      : null;

    function onDown(): void {
      scaleTo(0.95);
    }
    function onUp(): void {
      scaleTo(1);
    }
    function onMove(event: PointerEvent): void {
      if (!xTo || !yTo) return;
      const rect = el.getBoundingClientRect();
      const relX = event.clientX - (rect.left + rect.width / 2);
      const relY = event.clientY - (rect.top + rect.height / 2);
      const clampedX = Math.max(-magnetStrength, Math.min(magnetStrength, relX * 0.35));
      const clampedY = Math.max(-magnetStrength, Math.min(magnetStrength, relY * 0.35));
      xTo(clampedX);
      yTo(clampedY);
    }
    function onLeave(): void {
      scaleTo(1);
      xTo?.(0);
      yTo?.(0);
    }

    el.style.willChange = 'transform';
    el.addEventListener('pointerdown', onDown);
    el.addEventListener('pointerup', onUp);
    el.addEventListener('pointerleave', onLeave);
    if (magnetStrength) el.addEventListener('pointermove', onMove);

    registry.set(el, {
      cleanup: () => {
        el.removeEventListener('pointerdown', onDown);
        el.removeEventListener('pointerup', onUp);
        el.removeEventListener('pointerleave', onLeave);
        if (magnetStrength) el.removeEventListener('pointermove', onMove);
        gsap.killTweensOf(el);
      },
    });
  },
  unmounted(el) {
    registry.get(el)?.cleanup();
    registry.delete(el);
  },
};
