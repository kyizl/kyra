import { gsap } from 'gsap';

export const ease = {
  out: 'power3.out',
  in: 'power2.in',
  inOut: 'power2.inOut',
  spring: 'back.out(1.8)',
  elastic: 'elastic.out(1, 0.62)',
  snap: 'power4.out',
} as const;

export const duration = {
  instant: 0.12,
  fast: 0.18,
  base: 0.28,
  slow: 0.42,
  lazy: 0.6,
} as const;

let reduceMotionQuery: MediaQueryList | null = null;

function getReduceMotionQuery(): MediaQueryList | null {
  if (typeof window === 'undefined') return null;
  if (!reduceMotionQuery) {
    reduceMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
  }
  return reduceMotionQuery;
}

export function prefersReducedMotion(): boolean {
  return getReduceMotionQuery()?.matches ?? false;
}

export function motionEnabled(lowEndMode = false): boolean {
  return !lowEndMode && !prefersReducedMotion();
}

function target(el: Element): HTMLElement {
  return el as HTMLElement;
}

export function popEnter(el: Element, done: () => void): void {
  if (!motionEnabled()) {
    done();
    return;
  }
  gsap.fromTo(
    target(el),
    { autoAlpha: 0, scale: 0.92, y: 14, filter: 'blur(6px)' },
    {
      autoAlpha: 1,
      scale: 1,
      y: 0,
      filter: 'blur(0px)',
      duration: duration.slow,
      ease: ease.spring,
      onComplete: done,
    },
  );
}

export function popLeave(el: Element, done: () => void): void {
  if (!motionEnabled()) {
    done();
    return;
  }
  gsap.to(target(el), {
    autoAlpha: 0,
    scale: 0.95,
    y: 8,
    duration: duration.fast,
    ease: ease.in,
    onComplete: done,
  });
}

export function scrimEnter(el: Element, done: () => void): void {
  if (!motionEnabled()) {
    done();
    return;
  }
  gsap.fromTo(
    target(el),
    { autoAlpha: 0 },
    { autoAlpha: 1, duration: duration.base, ease: ease.out, onComplete: done },
  );
}

export function scrimLeave(el: Element, done: () => void): void {
  if (!motionEnabled()) {
    done();
    return;
  }
  gsap.to(target(el), {
    autoAlpha: 0,
    duration: duration.base,
    ease: ease.inOut,
    onComplete: done,
  });
}

export function dropEnter(el: Element, done: () => void): void {
  if (!motionEnabled()) {
    done();
    return;
  }
  gsap.fromTo(
    target(el),
    { autoAlpha: 0, y: -6, scale: 0.96, transformOrigin: 'top right' },
    {
      autoAlpha: 1,
      y: 0,
      scale: 1,
      duration: duration.fast,
      ease: ease.snap,
      onComplete: done,
    },
  );
}

export function dropLeave(el: Element, done: () => void): void {
  if (!motionEnabled()) {
    done();
    return;
  }
  gsap.to(target(el), {
    autoAlpha: 0,
    y: -4,
    scale: 0.97,
    duration: duration.instant,
    ease: ease.in,
    onComplete: done,
  });
}

export { gsap };
