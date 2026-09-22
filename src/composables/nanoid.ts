const pool = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

export function nanoid(size = 21): string {
  const randomBytes = crypto.getRandomValues(new Uint8Array(size));
  let id = '';
  for (const byte of randomBytes) id += pool[byte % pool.length];
  return id;
}
