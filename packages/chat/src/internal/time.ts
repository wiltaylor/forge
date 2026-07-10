export type ChatTime = string | number | Date;

export function formatTime(at: ChatTime): string {
  return new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(new Date(at));
}

/** Local-time calendar-day key for day-divider boundaries. */
export function dayKey(at: ChatTime): string {
  const d = new Date(at);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

export function formatDay(at: ChatTime): string {
  const now = new Date();
  const key = dayKey(at);
  if (key === dayKey(now)) return 'Today';
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (key === dayKey(yesterday)) return 'Yesterday';
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(new Date(at));
}

export function isoTime(at: ChatTime): string {
  return new Date(at).toISOString();
}

/** "4.2 MB" — mono captions want the space before the unit. */
export function formatBytes(size: number): string {
  const units = ['B', 'kB', 'MB', 'GB', 'TB'];
  let v = size;
  let i = 0;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i++;
  }
  return `${i === 0 ? v : v.toFixed(1)} ${units[i]}`;
}
