import { ipcMain } from 'electron';
import {
  acknowledgeUsageDisclosure,
  isLinked,
  isUsageDisclosureAcknowledged,
  startDiscordLink,
} from '../telemetry-service';

export function registerTelemetryHandlers(): void {
  ipcMain.handle('telemetry:is-linked', async () => isLinked());

  ipcMain.handle('telemetry:is-disclosure-acknowledged', async () =>
    isUsageDisclosureAcknowledged(),
  );

  ipcMain.handle('telemetry:acknowledge-disclosure', async () =>
    acknowledgeUsageDisclosure(),
  );

  ipcMain.on('telemetry:start-link', () => startDiscordLink());
}
