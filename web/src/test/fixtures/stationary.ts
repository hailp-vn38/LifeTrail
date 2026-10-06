import type { components } from "../../api/generated/lifetrail-v1";
import type { DailyView } from "../../api/queries/daily-view.query";

export const openStop: components["schemas"]["DailyStop"] = {
  id: "01900000-0000-7000-8000-000000000001:stop:0", kind: "stop",
  activity_revision: "01900000-0000-7000-8000-000000000001",
  observed_from_at: "2026-10-05T16:50:00Z", observed_until_at: "2026-10-05T17:20:00Z",
  observed_duration_s: 1800, actual_start_at: null, actual_end_at: null,
  start_boundary: "open", end_boundary: "open", full_duration_s: null,
  center: [106.7, 10.77], radius_m: 12, quality: "sufficient",
  source_record_count: 3, usable_record_count: 3, source_record_ids: [1, 2, 3],
  visible_from_at: "2026-10-05T16:50:00Z", visible_until_at: "2026-10-05T17:00:00Z",
  daily_observed_duration_s: 600, continues_before: false, continues_after: true,
};

export function stationaryView(): DailyView {
  return {
    device_id: "device-1", date: "2026-10-05", timezone: "Asia/Ho_Chi_Minh",
    processing_state: "processed", evidence_state: "sufficient",
    route: null, start: null, end: null, route_parts: [], timeline: [openStop], evidence_holes: [],
    summary: { point_count: 2, usable_point_count: 2, low_quality_point_count: 0, excluded_point_count: 0,
      distance_m: 0, duration_s: 600, trip_duration_s: 0, stop_duration_s: 600,
      gap_duration_s: 0, trip_count: 0, stop_count: 1, gap_count: 0,
      first_fix_at: "2026-10-05T16:50:00Z", last_fix_at: "2026-10-05T16:59:00Z" },
    processing: { state: "idle", data_freshness: "current", published_revision: "snapshot-1",
      input_generation: 1, timezone: "Asia/Ho_Chi_Minh", timezone_generation: 0,
      deferred_reason: null, failure_message: null },
  };
}
