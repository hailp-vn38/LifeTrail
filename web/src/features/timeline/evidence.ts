/**
 * User-facing explanations for the two distinct kinds of missing evidence.
 *
 * A GPS Gap is the absence of Raw observations and becomes a Timeline event. An
 * Evidence Hole is an interval that contains Raw observations which cannot
 * support reliable activity, and is reported separately. The two never share a
 * message, because uncertainty and absence are different facts about the Day.
 */
import type { DailyGap } from "../activity/model";
import { formatDuration, formatTimestamp } from "../../lib/format";

/** Headline and explanation for a GPS Gap, rendered as its own Timeline item. */
export function describeGap(
  gap: DailyGap,
  timezone: string,
): { title: string; subtitle: string } {
  return {
    title: "GPS Gap",
    subtitle: [
      `Thiếu quan sát GPS: ${formatTimestamp(gap.observed_from_at, timezone)} – ${formatTimestamp(gap.observed_until_at, timezone)}`,
      `Trong ngày: ${formatDuration(gap.daily_observed_duration_s)}`,
      "Không suy ra di chuyển, dừng lại hay đoạn đường nối qua khoảng này.",
    ].join(" · "),
  };
}

/** Why existing observations could not support activity, per hole reason. */
const HOLE_REASON: Record<string, string> = {
  insufficient_quality: "Có GPS nhưng chất lượng chưa đủ",
  insufficient_geometry: "Có GPS nhưng vị trí không hợp lý",
  ambiguous_activity: "Có GPS nhưng chưa xác định được hoạt động",
  unsupported_classification: "Có hoạt động nhưng chưa phân loại được",
};

/** One line per Evidence Hole, with its reason and Raw record count. */
export function describeEvidenceHoles(
  dailyView: {
    evidence_holes?: {
      observed_from_at: string;
      observed_until_at: string;
      reason: string;
      source_record_count: number;
    }[];
    timezone: string;
  },
): string[] {
  return (dailyView.evidence_holes ?? [])
    .slice()
    .sort((left, right) =>
      left.observed_from_at.localeCompare(right.observed_from_at),
    )
    .map(
      (hole) =>
        `${HOLE_REASON[hole.reason] ?? hole.reason}: ${formatTimestamp(hole.observed_from_at, dailyView.timezone)} – ${formatTimestamp(hole.observed_until_at, dailyView.timezone)} (${hole.source_record_count} bản ghi GPS)`,
    );
}