/**
 * User-facing explanations for the two distinct kinds of missing evidence.
 *
 * A GPS Gap is the absence of Raw observations and becomes a Timeline event. An
 * Evidence Hole is an interval that contains Raw observations which cannot
 * support reliable activity, and is reported separately. The two never share a
 * message, because uncertainty and absence are different facts about the Day.
 */
import type { DailyGap, EvidenceHole } from "../activity/model";
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

/**
 * Why existing observations could not support activity, per hole reason.
 *
 * Keyed by the generated `reason` union, so a reason the contract gains later
 * fails the typecheck here instead of silently falling through to a raw code.
 */
const HOLE_REASON: Record<EvidenceHole["reason"], string> = {
  insufficient_quality: "Có GPS nhưng chất lượng chưa đủ",
  insufficient_geometry: "Có GPS nhưng vị trí không hợp lý",
  ambiguous_activity: "Có GPS nhưng chưa xác định được hoạt động",
};

/** One line per Evidence Hole, with its reason and Raw record count. */
export function describeEvidenceHoles(
  dailyView: {
    evidence_holes?: EvidenceHole[];
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
        `${HOLE_REASON[hole.reason]}: ${formatTimestamp(hole.observed_from_at, dailyView.timezone)} – ${formatTimestamp(hole.observed_until_at, dailyView.timezone)} (${hole.source_record_count} bản ghi GPS)`,
    );
}

/**
 * The two poorer classification classes of the day, in one line.
 *
 * Each one also appears as its own Evidence Hole interval, but an interval says
 * *where* the unreliable records are, not how many the day withheld. Naming both
 * counts keeps the two reasons distinct: a record of poor quality is not an
 * impossible position.
 */
export function describeWithheldCoverage(summary: {
  low_quality_point_count?: number;
  excluded_point_count?: number;
}): string | null {
  const lowQuality = summary.low_quality_point_count ?? 0;
  const excluded = summary.excluded_point_count ?? 0;
  if (!lowQuality && !excluded) return null;
  return `${lowQuality} bản ghi GPS chất lượng thấp và ${excluded} bản ghi GPS có vị trí không hợp lý không được dùng để dựng đường đi.`;
}