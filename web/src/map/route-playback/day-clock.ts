import { shiftDay } from '../../lib/date';

/** Resolve local midnight with the Owner's IANA timezone, including DST days. */
function midnight(date: string, timezone: string): number {
  const target = Date.parse(`${date}T00:00:00Z`);
  let epoch = target;
  const formatter = new Intl.DateTimeFormat('en-CA', {
    timeZone: timezone, year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23',
  });
  for (let iteration = 0; iteration < 4; iteration++) {
    const parts = Object.fromEntries(formatter.formatToParts(epoch).map(p => [p.type, p.value]));
    const local = Date.parse(`${parts.year}-${parts.month}-${parts.day}T${parts.hour}:${parts.minute}:${parts.second}Z`);
    const correction = target - local;
    epoch += correction;
    if (!correction) break;
  }
  return epoch;
}

export function dayClockBounds(date: string, timezone: string) {
  return { startTimeMs: midnight(date, timezone), endTimeMs: midnight(shiftDay(date, 1), timezone) - 1 };
}
