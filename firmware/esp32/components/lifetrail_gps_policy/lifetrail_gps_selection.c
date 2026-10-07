#include "lifetrail_gps_policy_internal.h"

#include <math.h>

void lt_gps_policy_persist(lt_gps_policy_impl_t *impl, const lt_gps_record_t *record,
                    lt_gps_persist_reason_t reason) {
  if (impl->has_last_persisted && record->ts_ms <= impl->last_persisted.ts_ms) {
    return;
  }
  if (impl->persist_sink != NULL) {
    impl->persist_sink(impl->persist_sink_context, record, reason,
                       impl->motion_state);
  }
  if (impl->has_last_persisted && reason != LT_GPS_PERSIST_STATE_TRANSITION &&
      reason != LT_GPS_PERSIST_TRANSITION_BACKFILL &&
      reason != LT_GPS_PERSIST_SESSION_BOUNDARY && reason != LT_GPS_PERSIST_FIX_RECOVERY) {
    uint64_t interval = (uint64_t)(record->ts_ms - impl->last_persisted.ts_ms);
    if (impl->motion_state == LT_GPS_MOTION_MOVING) {
      impl->metrics.moving_persist_interval_total_ms += interval;
      impl->metrics.moving_persist_intervals++;
    } else if (impl->motion_state == LT_GPS_MOTION_STATIONARY) {
      impl->metrics.stationary_persist_interval_total_ms += interval;
      impl->metrics.stationary_persist_intervals++;
      if (interval > impl->metrics.max_stationary_persist_gap_ms)
        impl->metrics.max_stationary_persist_gap_ms = interval;
    }
  }
  impl->metrics.persisted_records++;
  impl->last_persisted = *record;
  impl->has_last_persisted = true;
  impl->next_heartbeat_ms = (uint64_t)record->ts_ms + impl->settings.stationary_interval_ms;
  if (reason == LT_GPS_PERSIST_HEADING) {
    impl->last_heading_persist_ms = (uint64_t)record->ts_ms;
  }
}

void lt_gps_policy_backfill(lt_gps_policy_impl_t *impl, uint64_t transition_ms) {
  size_t offset;
  size_t persisted = 0U;
  uint64_t first_ms = transition_ms > impl->settings.backfill_window_ms
      ? transition_ms - impl->settings.backfill_window_ms : 0U;
  for (offset = 0U; offset < impl->recent_count &&
                   persisted < impl->settings.backfill_max_records;
       offset++) {
    const lt_gps_record_t *record =
        &impl->recent[lt_gps_policy_recent_index(impl, offset)].record;
    if ((uint64_t)record->ts_ms >= first_ms &&
        (uint64_t)record->ts_ms < transition_ms &&
        (!impl->has_last_persisted || record->ts_ms > impl->last_persisted.ts_ms)) {
      lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_TRANSITION_BACKFILL);
      persisted++;
    }
  }
}

bool lt_gps_policy_heading_trigger(const lt_gps_policy_impl_t *impl,
                            const lt_gps_record_t *record) {
  double delta;
  if (!impl->has_last_persisted || !record->has_speed_mps ||
      !record->has_course_deg || !impl->last_persisted.has_course_deg ||
      record->speed_mps < impl->settings.heading_min_speed_mps ||
      (uint64_t)record->ts_ms - impl->last_heading_persist_ms <
          impl->settings.heading_min_interval_ms) {
    return false;
  }
  delta = fabs(record->course_deg - impl->last_persisted.course_deg);
  delta = delta > 180.0 ? 360.0 - delta : delta;
  return delta >= impl->settings.heading_degrees;
}

const lt_gps_recent_sample_t *lt_gps_policy_heartbeat_sample(const lt_gps_policy_impl_t *impl,
                                                uint64_t deadline_ms) {
  size_t offset;
  const lt_gps_recent_sample_t *chosen = NULL;
  uint64_t earliest = deadline_ms > impl->settings.heartbeat_selection_window_ms
      ? deadline_ms - impl->settings.heartbeat_selection_window_ms : 0U;
  for (offset = 0U; offset < impl->recent_count; offset++) {
    const lt_gps_recent_sample_t *sample = &impl->recent[lt_gps_policy_recent_index(impl, offset)];
    if (sample->jump || (uint64_t)sample->record.ts_ms < earliest ||
        (uint64_t)sample->record.ts_ms > deadline_ms) continue;
    chosen = sample;
  }
  return chosen;
}
