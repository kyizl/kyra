import { Buffer } from 'node:buffer';
import { createCipheriv, createDecipheriv, randomBytes } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const prefix = 'local:v1:';

function encryptionKey(directory: string, create: boolean): Buffer {
  const secretDirectory = join(directory, 'traffic-secrets');
  const path = join(secretDirectory, 'encryption.key');
  try {
    const key = readFileSync(path);
    if (key.length !== 32) throw new Error('Invalid local encryption key.');
    return key;
  } catch (error) {
    if (!create || (error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
  }
  mkdirSync(secretDirectory, { recursive: true, mode: 0o700 });
  const key = randomBytes(32);
  try {
    writeFileSync(path, key, { flag: 'wx', mode: 0o600 });
    return key;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'EEXIST') throw error;
    return encryptionKey(directory, false);
  }
}

export function encryptLocalSecret(directory: string, value: string): string {
  const key = encryptionKey(directory, true);
  const nonce = randomBytes(12);
  const cipher = createCipheriv('aes-256-gcm', key, nonce);
  const ciphertext = Buffer.concat([cipher.update(value, 'utf8'), cipher.final()]);
  const payload = Buffer.concat([nonce, ciphertext, cipher.getAuthTag()]);
  return prefix + payload.toString('base64');
}

export function isLocalSecret(value: string): boolean {
  return value.startsWith(prefix);
}

export function decryptLocalSecret(directory: string, value: string): string {
  if (!isLocalSecret(value)) throw new Error('Invalid encrypted API key.');
  const payload = Buffer.from(value.slice(prefix.length), 'base64');
  if (payload.length < 29) throw new Error('Invalid encrypted API key.');
  const decipher = createDecipheriv(
    'aes-256-gcm',
    encryptionKey(directory, false),
    payload.subarray(0, 12),
  );
  decipher.setAuthTag(payload.subarray(-16));
  return Buffer.concat([
    decipher.update(payload.subarray(12, -16)),
    decipher.final(),
  ]).toString('utf8');
}
