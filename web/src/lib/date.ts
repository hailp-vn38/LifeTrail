export function todayForOwner(timezone: string, instant = new Date()): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: timezone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(instant);
  const values = Object.fromEntries(
    parts
      .filter((part) => part.type !== "literal")
      .map((part) => [part.type, part.value]),
  );

  return `${values.year}-${values.month}-${values.day}`;
}

/** "2026-10-05" -> "5 thg 10, 2026" (vi-VN). Falls back to the raw value. */
export function formatDayLabel(date: string): string {
  const instant = new Date(`${date}T00:00:00`);
  if (Number.isNaN(instant.getTime())) return date;
  return new Intl.DateTimeFormat("vi-VN", {
    day: "numeric",
    month: "short",
    year: "numeric",
  }).format(instant);
}

/** Shift a YYYY-MM-DD date by whole days, returning YYYY-MM-DD. */
export function shiftDay(date: string, deltaDays: number): string {
  const instant = new Date(`${date}T00:00:00Z`);
  if (Number.isNaN(instant.getTime())) return date;
  instant.setUTCDate(instant.getUTCDate() + deltaDays);
  return instant.toISOString().slice(0, 10);
}
