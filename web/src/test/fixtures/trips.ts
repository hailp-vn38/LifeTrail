import type { components } from "../../api/generated/lifetrail-v1";
import type { DailyView } from "../../api/queries/daily-view.query";

export const REVISION = "01900000-0000-7000-8000-000000000010";

export const openTrip: components["schemas"]["DailyTrip"] = {
  id: `${REVISION}:trip:0`, kind: "trip", activity_revision: REVISION,
  observed_from_at: "2026-10-05T03:00:00Z", observed_until_at: "2026-10-05T03:20:00Z",
  observed_duration_s: 1200, actual_start_at: null, actual_end_at: null,
  start_boundary: "open", end_boundary: "open", full_duration_s: null,
  distance_m: 2204, quality: "sufficient", movement_segment_count: 1,
  movement_segments: [{
    id: `${REVISION}:trip:0:segment:0`, mode: "unknown", classification_confidence: 0, source: "raw",
    observed_from_at: "2026-10-05T03:00:00Z", observed_until_at: "2026-10-05T03:20:00Z",
    observed_duration_s: 1200, distance_m: 2204, quality: "sufficient",
    route_part_ids: [`${REVISION}:trip:0:segment:0:part:0`], source_record_count: 5,
  }],
  source_record_count: 5, usable_record_count: 5, source_record_ids: [10, 11, 12, 13, 14],
  visible_from_at: "2026-10-05T03:00:00Z", visible_until_at: "2026-10-05T03:20:00Z",
  daily_observed_duration_s: 1200, continues_before: false, continues_after: false,
};

export const rawPart: components["schemas"]["RoutePart"] = {
  id: `${REVISION}:trip:0:segment:0:part:0`, kind: "route_part",
  trip_id: openTrip.id, movement_segment_id: `${REVISION}:trip:0:segment:0`,
  source: "raw", mode: "unknown", classification_confidence: 0, observed_from_at: "2026-10-05T03:00:00Z",
  observed_until_at: "2026-10-05T03:20:00Z", distance_m: 2204, visible_distance_m: 2204,
  quality: "sufficient", source_record_count: 5,
  visible_from_at: "2026-10-05T03:00:00Z", visible_until_at: "2026-10-05T03:20:00Z",
  continues_before: false, continues_after: false,
  geometry: {
    type: "LineString",
    coordinates: [
      [106.7, 10.77], [106.7004, 10.7702], [106.7008, 10.7704],
      [106.7012, 10.7706], [106.7016, 10.7708],
    ],
  },
  vertex_distance_m: [0, 46.6, 93.2, 139.8, 186.4],
  progress_anchors: [
    { at: "2026-10-05T03:00:00Z", distance_m: 0 },
    { at: "2026-10-05T03:05:00Z", distance_m: 46.6 },
    { at: "2026-10-05T03:10:00Z", distance_m: 93.2 },
    { at: "2026-10-05T03:15:00Z", distance_m: 139.8 },
    { at: "2026-10-05T03:20:00Z", distance_m: 186.4 },
  ],
};

/** One processed day with a Trip, its Route Part and a Stop. */
export function tripView(): DailyView {
  return {
    device_id: "device-1", date: "2026-10-05", timezone: "Asia/Ho_Chi_Minh",
    processing_state: "processed", evidence_state: "sufficient",
    route: null, start: null, end: null,
    route_parts: [rawPart],
    timeline: [
      {
        id: "01900000-0000-7000-8000-000000000001:stop:0", kind: "stop",
        activity_revision: "01900000-0000-7000-8000-000000000001",
        observed_from_at: "2026-10-05T02:30:00Z", observed_until_at: "2026-10-05T02:58:00Z",
        observed_duration_s: 1680, actual_start_at: null, actual_end_at: "2026-10-05T02:58:00Z",
        start_boundary: "open", end_boundary: "confirmed", full_duration_s: null,
        center: [106.7, 10.77], radius_m: 12, quality: "sufficient",
        source_record_count: 3, usable_record_count: 3, source_record_ids: [1, 2, 3],
        visible_from_at: "2026-10-05T02:30:00Z", visible_until_at: "2026-10-05T02:58:00Z",
        daily_observed_duration_s: 1680, continues_before: false, continues_after: false,
      } as components["schemas"]["DailyStop"],
      openTrip,
    ],
    evidence_holes: [],
    summary: {
      point_count: 8, usable_point_count: 8, low_quality_point_count: 0, excluded_point_count: 0,
      distance_m: 2204, duration_s: 2880, trip_duration_s: 1200,
      stop_duration_s: 1680, gap_duration_s: 0, trip_count: 1, stop_count: 1, gap_count: 0,
      first_fix_at: "2026-10-05T02:30:00Z", last_fix_at: "2026-10-05T03:20:00Z",
    },
    processing: {
      state: "idle", data_freshness: "current", published_revision: "snapshot-2",
      input_generation: 2, timezone: "Asia/Ho_Chi_Minh", timezone_generation: 0,
      deferred_reason: null, failure_message: null,
    },
  } as DailyView;
}
