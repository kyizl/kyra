const regex = /^\[\d{2}:\d{2}:\d{2}\] \[[^\]]+\/INFO\]: (?:\[CHAT\] )?(.+)$/;
const username = '\\w{1,16}';

const patterns = {
  join: new RegExp(`(${username}) has joined! \\(\\d+/\\d+\\)`),
  quit: new RegExp(`(${username}) has quit! \\(\\d+/\\d+\\)`),
  quitDisconnected: new RegExp(`^(${username}) disconnected!$`),
  jartexJoin: new RegExp(
    `^BedWars \\? (${username}) has joined the game! \\(\\d+/\\d+\\)$`,
  ),
  jartexQuit: new RegExp(
    `^BedWars \\? (${username}) has left the game! \\(\\d+/\\d+\\)$`,
  ),
  whoPlain: /^((?:\[[A-Z0-9+_§]+\] )?\w+)(?:,\s*(?:\[[A-Z0-9+_§]+\] )?\w+)+$/,
  finalKill: /FINAL KILL/,
};

export function stripColorCodes(text: string): string {
  return text.replace(/[\u00A7\uFFFD][0-9A-FK-OR]/gi, '').replace(/[\u00A7\uFFFD]/g, '');
}

export type LogEvent =
  | { type: 'join'; name: string }
  | { type: 'quit'; name: string }
  | { type: 'who'; names: string[] }
  | { type: 'finalKill'; name: string };

export function parseLine(rawLine: string): LogEvent | null {
  const chatMatch = rawLine.match(regex);
  let message = (chatMatch ? chatMatch[1] : rawLine).trim();
  if (!message) return null;

  message = stripColorCodes(message);
  if (!message) return null;

  if (patterns.finalKill.test(message)) {
    const name = message.split(/\s+/)[0];
    if (/^\w{1,16}$/.test(name)) return { type: 'finalKill', name };
  }

  const joinMatch = message.match(patterns.join) ?? message.match(patterns.jartexJoin);
  if (joinMatch) return { type: 'join', name: joinMatch[1] };

  const quitMatch =
    message.match(patterns.quit) ??
    message.match(patterns.quitDisconnected) ??
    message.match(patterns.jartexQuit);
  if (quitMatch) return { type: 'quit', name: quitMatch[1] };

  if (patterns.whoPlain.test(message)) {
    const names = message.split(/\s*,\s*/).flatMap((part) => {
      const cleaned = part
        .replace(/\[[A-Z0-9+_§]+\]\s*/g, '')
        .replace(/[\u00A7\uFFFD]./g, '');
      return cleaned.match(/\w{1,16}/g) ?? [];
    });
    if (names.length >= 2) return { type: 'who', names };
  }

  return null;
}
