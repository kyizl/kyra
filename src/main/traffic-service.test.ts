import { Buffer } from 'node:buffer';
import {
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import process from 'node:process';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const storage = vi.hoisted(() => ({
  directory: '',
  available: true,
  backend: 'gnome_libsecret',
  encryptFails: false,
}));
vi.mock('electron', () => ({
  app: { getPath: () => storage.directory },
  safeStorage: {
    isEncryptionAvailable: () => storage.available,
    getSelectedStorageBackend: () => storage.backend,
    encryptString: vi.fn((value: string) => {
      if (storage.encryptFails) throw new Error('Credential store is locked.');
      return Buffer.from([...value].reverse().join(''));
    }),
    decryptString: vi.fn((value: Buffer) => [...value.toString()].reverse().join('')),
  },
}));

beforeEach(() => {
  vi.resetModules();
  storage.directory = mkdtempSync(join(tmpdir(), 'kyra-traffic-'));
  storage.available = true;
  storage.encryptFails = false;
  storage.backend = 'gnome_libsecret';
});

afterEach(() => rmSync(storage.directory, { recursive: true, force: true }));

describe('traffic settings', () => {
  it('defaults to unauthenticated middleware and isolates Jartex', async () => {
    const traffic = await import('./traffic-service');
    expect(traffic.getTrafficStatus()).toEqual({ mode: 'middleware', hasApiKey: false });
    expect(traffic.getStatsRoute('pika')).toMatchObject({
      base: 'https://proxy.kyizl.is-a.dev/api',
      headers: {},
    });
    traffic.saveTrafficApiKey('test-secret');
    expect(traffic.getStatsRoute('pika')).toMatchObject({
      base: 'https://stats.pika-network.net/api',
      headers: { 'X-Api-Key': 'test-secret' },
    });
    expect(traffic.getStatsRoute('jartex')).toMatchObject({
      base: 'https://stats.jartexnetwork.com/api',
      headers: {},
    });
    traffic.setTrafficMode('middleware');
    expect(traffic.getStatsRoute('pika').headers).toEqual({});
    expect(traffic.getTrafficStatus().hasApiKey).toBe(true);
  });

  it('encrypts, reloads, replaces, and removes the saved key', async () => {
    let traffic = await import('./traffic-service');
    const initialScope = traffic.getStatsRoute('pika').cacheScope;
    expect(traffic.saveTrafficApiKey('  first-secret  ')).toEqual({
      mode: 'direct',
      hasApiKey: true,
    });
    expect(traffic.getStatsRoute('pika').cacheScope).not.toBe(initialScope);
    expect(readFileSync(join(storage.directory, 'traffic.json'), 'utf8')).not.toContain(
      'first-secret',
    );
    vi.resetModules();
    traffic = await import('./traffic-service');
    expect(traffic.getStatsRoute('pika').headers).toEqual({
      'X-Api-Key': 'first-secret',
    });
    const savedScope = traffic.getStatsRoute('pika').cacheScope;
    traffic.saveTrafficApiKey('replacement-secret');
    expect(traffic.getStatsRoute('pika').cacheScope).not.toBe(savedScope);
    expect(traffic.getStatsRoute('pika').headers).toEqual({
      'X-Api-Key': 'replacement-secret',
    });
    expect(traffic.removeTrafficApiKey()).toEqual({
      mode: 'middleware',
      hasApiKey: false,
    });
    expect(
      JSON.parse(readFileSync(join(storage.directory, 'traffic.json'), 'utf8')),
    ).toEqual({ mode: 'middleware' });
    vi.resetModules();
    traffic = await import('./traffic-service');
    expect(traffic.getTrafficStatus()).toEqual({ mode: 'middleware', hasApiKey: false });
    expect(() => traffic.setTrafficMode('direct')).toThrow('Save an API key');
  });

  it('rejects invalid keys without changing the active settings', async () => {
    const traffic = await import('./traffic-service');
    for (const key of ['', '   ', 'a\r\nb', 'has spaces', '☃']) {
      expect(() => traffic.saveTrafficApiKey(key)).toThrow('valid API key');
    }
    expect(traffic.getTrafficStatus()).toEqual({ mode: 'middleware', hasApiKey: false });
  });

  it.each(['unavailable', 'basic_text', 'locked'])(
    'saves and reloads encrypted keys when system storage is %s',
    async (reason) => {
      storage.available = reason !== 'unavailable';
      storage.backend = reason === 'basic_text' ? 'basic_text' : 'gnome_libsecret';
      storage.encryptFails = reason === 'locked';
      let traffic = await import('./traffic-service');
      expect(traffic.saveTrafficApiKey('fallback-secret')).toEqual({
        mode: 'direct',
        hasApiKey: true,
      });
      const path = join(storage.directory, 'traffic.json');
      const config = readFileSync(path, 'utf8');
      expect(config).not.toContain('fallback-secret');
      expect((JSON.parse(config) as { encryptedApiKey: string }).encryptedApiKey).toMatch(
        /^local:v1:/,
      );
      const keyPath = join(storage.directory, 'traffic-secrets', 'encryption.key');
      expect(readFileSync(keyPath)).toHaveLength(32);
      if (process.platform !== 'win32') {
        expect(statSync(keyPath).mode & 0o777).toBe(0o600);
        expect(statSync(join(storage.directory, 'traffic-secrets')).mode & 0o777).toBe(
          0o700,
        );
      }
      vi.resetModules();
      traffic = await import('./traffic-service');
      expect(traffic.getStatsRoute('pika').headers).toEqual({
        'X-Api-Key': 'fallback-secret',
      });
      traffic.setTrafficMode('middleware');
      expect(traffic.getStatsRoute('pika').headers).toEqual({});
      traffic.setTrafficMode('direct');
      const original = (JSON.parse(config) as { encryptedApiKey: string })
        .encryptedApiKey;
      traffic.saveTrafficApiKey('fallback-secret');
      expect(
        (JSON.parse(readFileSync(path, 'utf8')) as { encryptedApiKey: string })
          .encryptedApiKey,
      ).not.toBe(original);
      traffic.saveTrafficApiKey('replacement-secret');
      vi.resetModules();
      traffic = await import('./traffic-service');
      expect(traffic.getStatsRoute('pika').headers).toEqual({
        'X-Api-Key': 'replacement-secret',
      });
      traffic.removeTrafficApiKey();
      expect(JSON.parse(readFileSync(path, 'utf8'))).toEqual({ mode: 'middleware' });
    },
  );

  it('detects tampered ciphertext and does not recreate a missing decryption key', async () => {
    storage.available = false;
    let traffic = await import('./traffic-service');
    traffic.saveTrafficApiKey('fallback-secret');
    const path = join(storage.directory, 'traffic.json');
    const config = JSON.parse(readFileSync(path, 'utf8')) as { encryptedApiKey: string };
    const prefix = 'local:v1:';
    const payload = Buffer.from(config.encryptedApiKey.slice(prefix.length), 'base64');
    payload[12] ^= 1;
    config.encryptedApiKey = prefix + payload.toString('base64');
    writeFileSync(path, JSON.stringify(config));
    vi.resetModules();
    traffic = await import('./traffic-service');
    expect(() => traffic.getStatsRoute('pika')).toThrow('Could not decrypt');
    traffic.saveTrafficApiKey('replacement-secret');
    const keyPath = join(storage.directory, 'traffic-secrets', 'encryption.key');
    unlinkSync(keyPath);
    expect(() => traffic.getStatsRoute('pika')).toThrow('Could not decrypt');
    expect(() => statSync(keyPath)).toThrow();
  });
});
