const pool = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
const randomByteLimit = 256 - (256 % pool.length);
const randomByteBucketSize = randomByteLimit / pool.length;

export function nanoid(size = 21): string {
  let id = '';
  while (id.length < size) {
    const randomBytes = crypto.getRandomValues(
      new Uint8Array(Math.max(32, size - id.length)),
    );
    for (const byte of randomBytes) {
      if (byte >= randomByteLimit) continue;
      id += pool[Math.floor(byte / randomByteBucketSize)];
      if (id.length === size) break;
    }
  }
  return id;
}
