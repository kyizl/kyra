import process from 'node:process';
import { electronApp, optimizer } from '@electron-toolkit/utils';
import { app } from 'electron';
import { disposeHttpClient } from './http-client';
import { registerAllIpcHandlers } from './ipc';
import { stopLogTail } from './ipc/log-handlers';
import { destroyRpcClient } from './ipc/rpc-handlers';
import { unregisterAllShortcuts } from './ipc/shortcuts-handlers';
import { dbg } from './logger';
import { startProxyManager, stopProxyManager } from './proxy-service';
import { disconnectSupportSocket } from './support-socket';
import { isUsageReportingActive, stopUsageReporting } from './usage-reporter';
import { createWindow, getMainWindow, stopCursorPoll } from './window';

export const api = 'https://overlay.kyizl.is-a.dev';

const disabledChromiumFeatures = [
  'HardwareMediaKeyHandling',
  'MediaSessionService',
  'AutofillServerCommunication',
  'OptimizationHints',
  'Translate',
  'BackForwardCache',
  'CalculateNativeWinOcclusion',
].join(',');

const heapCap = 192;

const processStart = performance.now();
const gotSingleInstanceLock = app.requestSingleInstanceLock();

if (!gotSingleInstanceLock) {
  app.quit();
} else {
  app.on('second-instance', () => {
    const win = getMainWindow();
    if (win) {
      if (win.isMinimized()) win.restore();
      win.focus();
    }
  });

  applyPlatformGpuSwitches();
  registerAllIpcHandlers();

  void app.whenReady().then(async () => {
    dbg.ipc(`app ready at +${(performance.now() - processStart).toFixed(0)}ms`);
    electronApp.setAppUserModelId('com.thebois.overlay');
    app.on('browser-window-created', (_, window) =>
      optimizer.watchWindowShortcuts(window),
    );

    if (process.platform === 'linux') await sleep(500);

    createWindow();
    dbg.ipc(`window created at +${(performance.now() - processStart).toFixed(0)}ms`);
    await startProxyManager();
    dbg.ipc(
      `proxy manager started at +${(performance.now() - processStart).toFixed(0)}ms`,
    );
  });

  app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') app.quit();
  });

  let usageFlushed = false;

  app.on('before-quit', (event) => {
    if (usageFlushed || !isUsageReportingActive()) return;

    event.preventDefault();
    usageFlushed = true;
    void stopUsageReporting().finally(() => app.quit());
  });

  app.on('will-quit', () => {
    void (async () => {
      stopCursorPoll();
      unregisterAllShortcuts();
      disconnectSupportSocket();
      await stopLogTail();
      await destroyRpcClient();
      await stopProxyManager();
      disposeHttpClient();
    })();
  });
}

function applyPlatformGpuSwitches(): void {
  if (process.platform === 'linux') {
    app.commandLine.appendSwitch('disable-gpu-vsync');
    app.commandLine.appendSwitch('disable-frame-rate-limit');
    app.commandLine.appendSwitch('disable-gpu-sandbox');
  }

  app.commandLine.appendSwitch('disable-features', disabledChromiumFeatures);
  app.commandLine.appendSwitch('disk-cache-size', '1048576');
  app.commandLine.appendSwitch('js-flags', `--max-old-space-size=${heapCap}`);
  app.commandLine.appendSwitch(
    'enable-hardware-overlays',
    'single-fullscreen,single-on-top',
  );
}

async function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
