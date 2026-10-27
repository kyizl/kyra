import type { TrafficMode } from '../traffic-service';
import { ipcMain } from 'electron';
import {
  getTrafficStatus,
  removeTrafficApiKey,
  saveTrafficApiKey,
  setTrafficMode,
} from '../traffic-service';

export function registerTrafficHandlers(): void {
  ipcMain.handle('traffic:get', () => getTrafficStatus());
  ipcMain.handle('traffic:set-mode', (_, mode: TrafficMode) => setTrafficMode(mode));
  ipcMain.handle('traffic:save-api-key', (_, apiKey: string) =>
    saveTrafficApiKey(apiKey),
  );
  ipcMain.handle('traffic:remove-api-key', () => removeTrafficApiKey());
}
