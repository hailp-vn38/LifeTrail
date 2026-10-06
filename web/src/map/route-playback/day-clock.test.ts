import { describe, expect, it } from 'vitest';
import { dayClockBounds } from './day-clock';

describe('Owner calendar-day clock', () => {
  it('starts at local midnight in Vietnam', () => {
    const bounds = dayClockBounds('2026-10-05', 'Asia/Ho_Chi_Minh');
    expect(new Date(bounds.startTimeMs).toISOString()).toBe('2026-10-04T17:00:00.000Z');
    expect(bounds.endTimeMs - bounds.startTimeMs + 1).toBe(86_400_000);
  });
  it('respects a 23-hour daylight-saving day', () => {
    const bounds = dayClockBounds('2026-03-08', 'America/New_York');
    expect(bounds.endTimeMs - bounds.startTimeMs + 1).toBe(23 * 3_600_000);
  });
  it('respects a 25-hour daylight-saving day', () => {
    const bounds = dayClockBounds('2026-11-01', 'America/New_York');
    expect(bounds.endTimeMs - bounds.startTimeMs + 1).toBe(25 * 3_600_000);
  });
});
