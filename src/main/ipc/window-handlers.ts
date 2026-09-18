import process from 'node:process';
import { clipboard, ipcMain, shell } from 'electron';
import { dbg } from '../logger';
import {
  allowLinuxMinimize,
  getMainWindow,
  isIgnoringMouseEvents,
  setIgnoringMouseEvents,
  setWindowAlwaysOnTop,
  startCursorPoll,
  stopCursorPoll,
} from '../window';

function shouldGrowWindow(currentWidth: number, desiredContentWidth: number): boolean {
  const widthDelta = desiredContentWidth - currentWidth;
  return widthDelta >= 24 && widthDelta <= 480;
}

export function registerWindowHandlers(): void {
  ipcMain.on('win:minimize', () => {
    if (process.platform === 'linux') allowLinuxMinimize();
    getMainWindow()?.minimize();
  });

  ipcMain.on('win:close', () => getMainWindow()?.close());

  ipcMain.on('win:toggle-minimize', () => {
    const win = getMainWindow();
    if (win?.isMinimized()) {
      win.showInactive();
      return;
    }
    if (process.platform === 'linux') allowLinuxMinimize();
    win?.minimize();
  });

  ipcMain.on('win:open-external', (_, url: string) => void shell.openExternal(url));

  ipcMain.on('win:set-always-on-top', (_, enabled: boolean) => {
    setWindowAlwaysOnTop(enabled);
  });

  ipcMain.on('win:set-ignore-mouse', (_, ignore: boolean) => {
    const win = getMainWindow();
    if (!win) return;

    const previous = isIgnoringMouseEvents();
    if (previous === ignore) return;

    setIgnoringMouseEvents(ignore);

    if (ignore) {
      win.setIgnoreMouseEvents(true, { forward: true });
      startCursorPoll();
    } else {
      win.setIgnoreMouseEvents(false);
      stopCursorPoll();
    }
  });

  ipcMain.on('win:fit-content-width', (_, desiredContentWidth: number) => {
    const win = getMainWindow();
    if (!win) return;
    const { width: currentWidth, height } = win.getContentBounds();
    const width = Math.round(desiredContentWidth);
    if (!shouldGrowWindow(currentWidth, width)) return;
    dbg.ipc(`win:fit-content-width - ${currentWidth}px -> ${width}px`);
    win.setContentSize(width, height);
  });

  ipcMain.handle('win:screenshot', async () => {
    const image = await getMainWindow()?.webContents.capturePage();
    if (image) clipboard.writeImage(image);
  });
}
