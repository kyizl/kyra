import { createRouter, createWebHashHistory } from 'vue-router';

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'Home',
      component: async () => import('@renderer/views/Home.vue'),
    },
    {
      path: '/setup',
      name: 'Setup',
      component: async () => import('@renderer/views/Setup.vue'),
    },
    {
      path: '/nicks',
      name: 'Nicks',
      component: async () => import('@renderer/views/Nicks.vue'),
    },
    {
      path: '/settings',
      name: 'Settings',
      component: async () => import('@renderer/views/Settings.vue'),
    },
    {
      path: '/theme',
      name: 'Theme',
      component: async () => import('@renderer/views/Theme.vue'),
    },
    {
      path: '/queue',
      name: 'Queue',
      component: async () => import('@renderer/views/Queue.vue'),
    },
    {
      path: '/support',
      name: 'Support',
      component: async () => import('@renderer/views/Support.vue'),
    },
    {
      path: '/benchmark',
      name: 'Benchmark',
      component: async () => import('@renderer/components/BenchmarkPanel.vue'),
    },
  ],
});

export default router;
