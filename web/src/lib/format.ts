export function formatDistance(distanceMeters: number): string {
  if (distanceMeters >= 1000) return `${(distanceMeters / 1000).toFixed(2)} km`;
  return `${Math.round(distanceMeters)} m`;
}

export function formatDuration(durationSeconds: number): string {
  const hours = Math.floor(durationSeconds / 3600);
  const minutes = Math.floor((durationSeconds % 3600) / 60);
  if (hours) return `${hours} giờ ${minutes} phút`;
  return `${minutes} phút`;
}

export function formatTimestamp(timestamp: string | null, timezone: string): string {
  if (!timestamp) return "—";
  return new Intl.DateTimeFormat("vi-VN", {
    timeZone: timezone,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(timestamp));
}

/** Locale-aware integer formatting, e.g. 13428 -> "13.428" (vi-VN). */
export function formatCount(value: number): string {
  return new Intl.NumberFormat("vi-VN", { maximumFractionDigits: 0 }).format(value);
}
