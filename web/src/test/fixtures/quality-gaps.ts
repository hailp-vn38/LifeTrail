/**
 * Published Daily View with a GPS Gap and an Evidence Hole.
 *
 * `gap` is a Timeline event for an interval without observations;
 * `evidence_holes` is separate coverage metadata for intervals that contain
 * unreliable records. They are deliberately separate fields.
 */
import type { components } from "../../api/generated/lifetrail-v1";
import type { DailyView } from "../../api/queries/daily-view.query";

type DailyGap = components["schemas"]["DailyGap"];

const gap: DailyGap = {
  id: "rev:gap:0",
  kind: "gap",
  activity_revision: "rev",
  observed_from_at: "2026-10-05T03:20:00Z",
  observed_until_at: "2026-10-05T03:31:00Z",
  observed_duration_s: 660,
  visible_from_at: "2026-10-05T03:20:00Z",
  visible_until_at: "2026-10-05T03:31:00Z",
  daily_observed_duration_s: 660,
  continues_before: false,
  continues_after: false,
};

export function qualityGapView(): DailyView {
  return {
    device_id: "device-1",
    date: "2026-10-05",
    timezone: "Asia/Ho_Chi_Minh",
    processing_state: "processed",
    processing: {
      state: "idle",
      data_freshness: "current",
      published_revision: "rev",
      input_generation: 1,
      timezone: "Asia/Ho_Chi_Minh",
      timezone_generation: 1,
      deferred_reason: null,
      failure_message: null,
    },
    evidence_state: "partial",
    provenance: {
      manifest_version: "manifest",
      source_raw_generation: 1,
      processed_through_generation: 1,
      processing_target: "quality-gaps-v1",
      processing_target_generation: 3,
      timezone_generation: 1,
      reducer_version: 2,
      projection_schema_version: 1,
    },
    route_parts: [],
    timeline: [gap],
    evidence_holes: [
      {
        observed_from_at: "2026-10-05T02:00:00Z",
        observed_until_at: "2026-10-05T02:11:00Z",
        reason: "insufficient_geometry",
        source_record_count: 7,
      },
      {
        observed_from_at: "2026-10-05T04:00:00Z",
        observed_until_at: "2026-10-05T04:04:00Z",
        reason: "insufficient_quality",
        source_record_count: 5,
      },
    ],
    summary: {
      point_count: 24,
      usable_point_count: 12,
      low_quality_point_count: 5,
      excluded_point_count: 7,
      distance_m: 0,
      duration_s: 660,
      first_fix_at: "2026-10-05T02:00:00Z",
      last_fix_at: "2026-10-05T04:04:00Z",
      trip_duration_s: 0,
      stop_duration_s: 0,
      gap_duration_s: 660,
      trip_count: 0,
      stop_count: 0,
      gap_count: 1,
    },
    route: null,
    start: null,
    end: null,
  };
}