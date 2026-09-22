import { createBot } from 'mineflayer';
import type { Bot } from 'mineflayer';
import { PikaClient } from '../src';

const username = process.env.KYRA_TEST_USERNAME ?? 'KxrpyTyx';
const password = process.env.KYRA_TEST_PASSWORD;
const host = process.env.KYRA_TEST_HOST ?? '127.0.0.1';
const port = Number(process.env.KYRA_TEST_PORT ?? '25566');
const version = process.env.KYRA_TEST_VERSION ?? '1.8.9';
const timeoutMs = Number(process.env.KYRA_TEST_TIMEOUT_MS ?? '30000');

if (!password) {
  throw new Error(
    'KYRA_TEST_PASSWORD is required; use a disposable Pika account password',
  );
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function waitForSpawn(bot: Bot): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      bot.removeListener('spawn', onSpawn);
      reject(new Error(`timed out waiting for spawn after ${timeoutMs}ms`));
    }, timeoutMs);
    const onSpawn = () => {
      clearTimeout(timer);
      resolve();
    };
    bot.once('spawn', onSpawn);
  });
}

async function retry<T>(operation: () => Promise<T>, attempts: number): Promise<T> {
  let lastError: unknown;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;
      console.error(
        `[client] attempt ${attempt}/${attempts} failed: ${error instanceof Error ? error.message : String(error)}`,
      );
      if (attempt < attempts) await delay(1500);
    }
  }
  throw lastError instanceof Error ? lastError : new Error(String(lastError));
}

async function main(): Promise<void> {
  const bot = createBot({
    host,
    port,
    username,
    auth: 'offline',
    version,
    hideErrors: false,
  });

  bot.on('messagestr', (message) => {
    if (message.trim()) console.log(`[chat] ${message}`);
  });
  bot.on('kicked', (reason) => console.error(`[kicked] ${reason}`));
  bot.on('error', (error) => console.error(`[client-error] ${error.message}`));
  bot.on('end', (reason) => console.log(`[end] ${reason ?? 'connection closed'}`));

  try {
    console.log(
      `[client] connecting username=${username} version=${version} ${host}:${port}`,
    );
    await waitForSpawn(bot);
    console.log('[client] spawn reached');

    bot.chat(`/register ${password} ${password}`);
    await delay(1500);
    bot.chat(`/login ${password}`);
    await delay(3000);

    const pika = new PikaClient(bot, { antiAfk: true });
    pika.on('debug', (event, data) =>
      console.log(`[pika] ${event} ${JSON.stringify(data ?? {})}`),
    );
    await retry(() => pika.connectToBedWars(), 3);
    console.log('[client] BedWars server selected');

    if (process.env.KYRA_TEST_STOP_AT_BEDWARS_SERVER === '1') {
      await delay(5000);
      console.log(`[client] server-only stable=${!bot.ended}`);
      return;
    }

    bot.chat('/bedwars-1');
    console.log('[client] sent /bedwars-1');
    await delay(10000);

    const visiblePlayers = Object.keys(bot.players);
    console.log(`[client] stable=${!bot.ended} visiblePlayers=${visiblePlayers.length}`);
    if (bot.ended)
      throw new Error('client disconnected before the BedWars observation window ended');
  } finally {
    bot.quit('Kyra authenticated test complete');
  }
}

main().catch((error) => {
  console.error(`[fatal] ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
});
