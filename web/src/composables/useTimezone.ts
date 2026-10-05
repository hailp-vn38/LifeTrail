import { computed, type ComputedRef } from "vue";

/**
 * Resolve the effective timezone for display: an explicit device timezone
 * wins, otherwise the browser's local timezone is used.
 */
export function useTimezone(deviceTimezone?: string | null): ComputedRef<string> {
  return computed(() => {
    if (deviceTimezone) return deviceTimezone;
    try {
      return Intl.DateTimeFormat().resolvedOptions().timeZone ?? "UTC";
    } catch {
      return "UTC";
    }
  });
}
