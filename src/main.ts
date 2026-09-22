import { createPinia } from 'pinia';
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate';
import { createApp } from 'vue';
import App from './App.vue';
import { vPress } from './directives/press';
import { mark } from './lib/perf-bus';
import router from './router';
import { perfTimingPlugin } from './store/perf-plugin';
import { installTauriApi } from './tauri-api';
import './assets/global.css';

declare global {
  interface Window {
    __pinia: ReturnType<typeof createPinia>;
  }
}

mark('renderer-script-start');

installTauriApi();

document.addEventListener(
  'contextmenu',
  (event) => {
    event.preventDefault();
  },
  { capture: true },
);

document.addEventListener(
  'dragstart',
  (event) => {
    event.preventDefault();
  },
  { capture: true },
);

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);
pinia.use(perfTimingPlugin);

window.__pinia = pinia;

const app = createApp(App);

app.directive('press', vPress);

app.use(pinia).use(router).mount('#app');

mark('renderer-mounted');
