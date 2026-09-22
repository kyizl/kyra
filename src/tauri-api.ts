import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type Unsubscribe = () => void;

interface DialogResult {
  canceled: boolean;
  filePaths: string[];
}

interface ProxyStatus {
  pika: {
    running: boolean;
    port: number;
    bindHost: string;
    clientCount: number;
    error: string | null;
  };
  jartex: {
    running: boolean;
    port: number;
    bindHost: string;
    clientCount: number;
    error: string | null;
  };
}

function subscribe<T>(event: string, callback: (payload: T) => void): Unsubscribe {
  let active = true;
  let unsubscribe: Unsubscribe | undefined;
  void listen<T>(event, ({ payload }) => {
    if (active) callback(payload);
  }).then((remove) => {
    unsubscribe = remove;
    if (!active) remove();
  });
  return () => {
    active = false;
    unsubscribe?.();
  };
}

function createTauriApi() {
  return {
    platform: navigator.platform.toLowerCase(),
    pika: {
      fetch: async (
        username: string,
        interval = 'total',
        mode = 'ALL_MODES',
        concurrent = false,
      ) =>
        invoke('stats_fetch', { network: 'pika', username, interval, mode, concurrent }),
      stats: async (username: string, interval: string, mode: string) =>
        invoke('stats_get', { network: 'pika', username, interval, mode }),
      clan: async (name: string) => invoke('stats_clan', { network: 'pika', name }),
    },
    jartex: {
      fetch: async (
        username: string,
        interval = 'total',
        mode = 'ALL_MODES',
        concurrent = false,
      ) =>
        invoke('stats_fetch', {
          network: 'jartex',
          username,
          interval,
          mode,
          concurrent,
        }),
      stats: async (username: string, interval: string, mode: string) =>
        invoke('stats_get', { network: 'jartex', username, interval, mode }),
      clan: async (name: string) => invoke('stats_clan', { network: 'jartex', name }),
    },
    win: {
      minimize: () => void invoke('window_minimize'),
      close: () => void invoke('window_close'),
      toggleMinimize: () => void invoke('window_toggle_minimize'),
      openExternal: (url: string) => void invoke('window_open_external', { url }),
      setAlwaysOnTop: (enabled: boolean) =>
        void invoke('window_set_always_on_top', { enabled }),
      focus: () => void invoke('window_focus'),
      screenshot: async () => invoke('window_screenshot'),
      fitContentWidth: async (desiredContentWidth: number) =>
        void invoke('window_fit_content_width', { desiredWidth: desiredContentWidth }),
      setIgnoreMouse: (ignore: boolean) =>
        void invoke('window_set_ignore_mouse', { ignore }),
      startCursorPoll: () =>
        void invoke('cursor_poll_start').catch((error: unknown) => {
          console.error('Cursor polling failed', error);
        }),
      stopCursorPoll: () =>
        void invoke('cursor_poll_stop').catch((error: unknown) => {
          console.error('Cursor polling stop failed', error);
        }),
      onForwardedMove: (callback: (x: number | null, y: number | null) => void) => {
        return subscribe<[number, number] | null>('cursor:forwarded-move', (position) => {
          callback(position?.[0] ?? null, position?.[1] ?? null);
        });
      },
    },
    clipboard: {
      setText: async (text: string) => invoke('clipboard_set_text', { text }),
    },
    app: {
      getPath: async (name: string) => invoke<string>('app_get_path', { name }),
      findLunarLog: async () => invoke<string>('app_find_lunar_log'),
      openImageDialog: async () => invoke<DialogResult>('app_open_image_dialog'),
      readFileBase64: async (filePath: string) =>
        invoke<string>('app_read_file_base64', { filePath }),
    },
    log: {
      setPath: (path: string | null) => void invoke('log_set_path', { path }),
      checkPath: async (path: string) => invoke<boolean>('log_check_path', { path }),
      openDialog: async () => invoke<DialogResult>('log_open_dialog'),
      onLine: (callback: (line: string) => void) =>
        subscribe<string[]>('log:line', (lines) => lines.forEach(callback)),
    },
    shortcuts: {
      register: async (shortcuts: string[]) =>
        invoke<void>('shortcuts_register', { shortcuts }),
      onFired: (callback: (shortcut: string) => void) =>
        subscribe<string>('shortcut:fired', callback),
    },
    proxy: {
      getStatus: async () => invoke<ProxyStatus>('proxy_get_status'),
      start: async () => invoke<ProxyStatus>('proxy_start'),
      configureAndStart: async (
        pikaPort: number,
        jartexPort: number,
        bindHost: '0.0.0.0' | '127.0.0.1',
      ) =>
        invoke<ProxyStatus>('proxy_configure_and_start', {
          pikaPort,
          jartexPort,
          bindHost,
        }),
      stop: async () => invoke<ProxyStatus>('proxy_stop'),
      setPort: async (network: 'pikanetwork' | 'jartexnetwork', port: number) =>
        invoke<void>('proxy_set_port', { network, port }),
      setBindHost: async (bindHost: '0.0.0.0' | '127.0.0.1') =>
        invoke<void>('proxy_set_bind_host', { bindHost }),
      onEvent: (callback: (event: unknown) => void) => subscribe('proxy:event', callback),
    },
    telemetry: {
      isLinked: async () => invoke<boolean>('telemetry_is_linked'),
      startLink: () => {
        void invoke('telemetry_start_link').catch((error: unknown) => {
          console.error('Telemetry link failed', error);
        });
      },
      onEvent: (callback: (event: unknown) => void) =>
        subscribe('telemetry:event', callback),
    },
    rpc: {
      setEnabled: (enabled: boolean) => {
        void invoke('rpc_set_enabled', { enabled });
      },
      setActive: (active: boolean) => {
        void invoke('rpc_set_active', { active });
      },
      setNetwork: (network: string) => {
        void invoke('rpc_set_network', { network });
      },
      destroy: () => {
        void invoke('rpc_destroy');
      },
    },
    auth: {
      start: async () => invoke('auth_start'),
      poll: async (
        deviceCode: string,
        intervalSeconds: number,
        expiresInSeconds: number,
      ) => invoke('auth_poll', { deviceCode, intervalSeconds, expiresInSeconds }),
      status: async () => invoke('auth_status'),
      refresh: async () => invoke('auth_refresh'),
      logout: async () => invoke('auth_logout'),
    },
    updater: {
      check: () => {
        void invoke('updater_check').catch((error: unknown) => {
          console.error('Updater check failed', error);
        });
      },
      install: () => {
        void invoke('updater_install').catch((error: unknown) => {
          console.error('Updater install failed', error);
        });
      },
      onStatus: (callback: (payload: unknown) => void) =>
        subscribe('updater:status', callback),
    },
    perf: {
      dump: async (rendererSnapshot: unknown) =>
        invoke<string>('perf_dump', { rendererSnapshot }),
      startTrace: async () => invoke<boolean>('perf_start_trace'),
      stopTrace: async () => invoke<string | null>('perf_stop_trace'),
      openLogDir: async () => invoke<string>('perf_open_log_dir'),
    },
    support: {
      list: async () => invoke('support_list'),
      create: async (subject: string, message: string) =>
        invoke('support_create', { subject, message }),
      get: async (id: string) => invoke('support_get', { id }),
      reply: async (id: string, message: string) =>
        invoke('support_reply', { id, message }),
      connectSocket: () => {
        void invoke('support_socket_connect').catch((error: unknown) => {
          console.error('Support socket connection failed', error);
        });
      },
      disconnectSocket: () => {
        void invoke('support_socket_disconnect').catch((error: unknown) => {
          console.error('Support socket disconnection failed', error);
        });
      },
      onSocketEvent: (callback: (event: unknown) => void) =>
        subscribe('support:socket-event', callback),
    },
  };
}

export function installTauriApi(): void {
  window.api = createTauriApi() as Window['api'];
}
