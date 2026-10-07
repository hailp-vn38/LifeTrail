#include "lifetrail_gps_policy_internal.h"

#include <math.h>
#include <string.h>

_Static_assert(sizeof(lt_gps_policy_impl_t) <= sizeof(lt_gps_policy_t),
               "policy state is too small");

static lt_gps_policy_impl_t *policy_impl(lt_gps_policy_t *policy) {
  return (lt_gps_policy_impl_t *)&policy->state;
}

static const lt_gps_policy_impl_t *policy_impl_const(const lt_gps_policy_t *policy) {
  return (const lt_gps_policy_impl_t *)&policy->state;
}

lt_gps_policy_settings_t lt_gps_policy_default_settings(void) {
  return (lt_gps_policy_settings_t){
      .persist_min_interval_ms = UINT64_C(2000),
      .moving_max_interval_ms = UINT64_C(3000),
      .moving_distance_m = 15.0,
      .heading_degrees = 25.0,
      .heading_min_speed_mps = 2.0,
      .heading_min_interval_ms = UINT64_C(1000),
      .candidate_interval_ms = UINT64_C(30000),
      .stationary_interval_ms = UINT64_C(120000),
      .candidate_detect_window_ms = UINT64_C(30000),
      .stationary_confirm_ms = UINT64_C(120000),
      .candidate_speed_ratio_pct = 80U,
      .candidate_displacement_m = 10.0,
      .moving_speed_mps = 1.0,
      .stationary_speed_mps = 0.6,
      .stationary_drift_absorb_m = 10.0,
      .stationary_move_distance_m = 20.0,
      .jump_speed_mps = 70.0,
      .backfill_window_ms = UINT64_C(5000),
      .backfill_max_records = 6U,
      .evidence_max_interval_ms = UINT64_C(2000),
      .heartbeat_selection_window_ms = UINT64_C(5000),
  };
}

void lt_gps_policy_init(lt_gps_policy_t *policy,
                        const lt_gps_policy_settings_t *settings,
                        lt_gps_persist_sink_t persist_sink,
                        void *persist_sink_context) {
  lt_gps_policy_impl_t *impl = policy_impl(policy);
  memset(impl, 0, sizeof(*impl));
  impl->settings = settings == NULL ? lt_gps_policy_default_settings() : *settings;
  impl->persist_sink = persist_sink;
  impl->persist_sink_context = persist_sink_context;
  impl->motion_state = LT_GPS_MOTION_MOVING;
}

void lt_gps_policy_process(lt_gps_policy_t *policy,
                           const lt_gps_record_t *record) {
  lt_gps_policy_impl_t *impl = policy_impl(policy);
  bool jump;
  bool resumed_after_gap = false;
  uint64_t now_ms;
  uint64_t elapsed_since_persist;
  if (record == NULL || record->ts_ms <= 0 || !isfinite(record->lat) ||
      !isfinite(record->lon) || record->lat < -90.0 || record->lat > 90.0 ||
      record->lon < -180.0 || record->lon > 180.0 ||
      (record->has_speed_mps && (!isfinite(record->speed_mps) || record->speed_mps < 0.0)) ||
      (record->has_course_deg && (!isfinite(record->course_deg) ||
        record->course_deg < 0.0 || record->course_deg >= 360.0))) {
    return;
  }
  impl->metrics.raw_navigation_epochs++;
  if (impl->has_last_observed && record->ts_ms <= impl->last_observed.ts_ms) {
    impl->metrics.rejected_timestamps++;
    return;
  }
  now_ms = (uint64_t)record->ts_ms;
  if (impl->has_last_observed &&
      now_ms - (uint64_t)impl->last_observed.ts_ms > impl->settings.evidence_max_interval_ms) {
    resumed_after_gap = true;
    /* Preserve the final real observation before a no-fix interval. Its GPS
       timestamp remains before the gap; no observation is created inside it. */
    lt_gps_policy_finish(policy);
    impl->metrics.no_fix_duration_ms += now_ms - (uint64_t)impl->last_observed.ts_ms - UINT64_C(1000);
    impl->consecutive_moving = 0U;
    impl->candidate_stable = false;
    impl->recent_count = 0U;
    impl->recent_start = 0U;
  }
  jump = lt_gps_policy_is_jump(impl, record);
  lt_gps_policy_append_recent(impl, record, jump);
  impl->last_observed = *record;
  impl->has_last_observed = true;
  lt_gps_policy_update_moving_counter(impl, record, jump);

  if (!impl->has_last_persisted) {
    lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_PERIODIC);
    return;
  }
  elapsed_since_persist = now_ms - (uint64_t)impl->last_persisted.ts_ms;

  if (resumed_after_gap && !jump) {
    /* A real observation bounds fix recovery immediately, rather than waiting
       up to a stationary heartbeat before exposing the end of the GPS Gap. */
    lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_FIX_RECOVERY);
    return;
  }

  if (impl->motion_state != LT_GPS_MOTION_MOVING && lt_gps_policy_moving_evidence(impl, record)) {
    impl->motion_state = LT_GPS_MOTION_MOVING;
    impl->metrics.state_transitions++;
    impl->state_since_ms = now_ms;
    impl->has_stationary_center = false;
    lt_gps_policy_backfill(impl, now_ms);
    lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_STATE_TRANSITION);
    return;
  }
  if (impl->motion_state == LT_GPS_MOTION_MOVING && !jump &&
      now_ms - impl->state_since_ms >= impl->settings.candidate_detect_window_ms &&
      lt_gps_policy_candidate_condition(impl, now_ms)) {
    impl->motion_state = LT_GPS_MOTION_CANDIDATE_STOP;
    impl->metrics.state_transitions++;
    impl->state_since_ms = now_ms;
    impl->stable_since_ms = now_ms;
    impl->candidate_stable = true;
    lt_gps_policy_initialize_center(impl);
    lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_STATE_TRANSITION);
    return;
  }
  if (impl->motion_state == LT_GPS_MOTION_CANDIDATE_STOP) {
    bool stable = !jump && lt_gps_policy_candidate_condition(impl, now_ms);
    if (!stable) impl->candidate_stable = false;
    if (stable && !impl->candidate_stable) {
      impl->candidate_stable = true;
      impl->stable_since_ms = now_ms;
    }
  }
  if (impl->motion_state == LT_GPS_MOTION_CANDIDATE_STOP &&
      impl->candidate_stable &&
      now_ms - impl->stable_since_ms >= impl->settings.stationary_confirm_ms) {
    impl->motion_state = LT_GPS_MOTION_STATIONARY;
    impl->metrics.state_transitions++;
    impl->state_since_ms = now_ms;
    lt_gps_policy_initialize_center(impl);
    lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_STATE_TRANSITION);
    return;
  }

  if (impl->motion_state == LT_GPS_MOTION_MOVING) {
    if (lt_gps_policy_heading_trigger(impl, record)) {
      lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_HEADING);
    } else if (elapsed_since_persist >= impl->settings.persist_min_interval_ms &&
               lt_gps_policy_distance_m(&impl->last_persisted, record) >=
                   impl->settings.moving_distance_m) {
      lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_DISTANCE);
    } else if (elapsed_since_persist >= impl->settings.persist_min_interval_ms &&
               elapsed_since_persist >= impl->settings.moving_max_interval_ms) {
      lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_PERIODIC);
    }
  } else if (impl->motion_state == LT_GPS_MOTION_CANDIDATE_STOP) {
    if (elapsed_since_persist >= impl->settings.candidate_interval_ms) {
      lt_gps_policy_persist(impl, record, LT_GPS_PERSIST_PERIODIC);
    }
  } else if (now_ms >= impl->next_heartbeat_ms && impl->settings.stationary_interval_ms > 0U) {
    /* Advance missed grid deadlines without replaying old coordinates. */
    if (now_ms > impl->next_heartbeat_ms + impl->settings.heartbeat_selection_window_ms) {
      uint64_t missed = (now_ms - impl->next_heartbeat_ms) / impl->settings.stationary_interval_ms;
      impl->next_heartbeat_ms += missed * impl->settings.stationary_interval_ms;
    }
    const lt_gps_recent_sample_t *heartbeat = lt_gps_policy_heartbeat_sample(
        impl, impl->next_heartbeat_ms);
    impl->next_heartbeat_ms += impl->settings.stationary_interval_ms;
    if (heartbeat != NULL) {
      lt_gps_policy_persist(impl, &heartbeat->record, LT_GPS_PERSIST_HEARTBEAT);
      if (impl->has_stationary_center &&
          lt_gps_policy_center_distance_m(impl, &heartbeat->record) <=
              impl->settings.stationary_drift_absorb_m) {
        impl->stationary_center_lat = heartbeat->record.lat;
        impl->stationary_center_lon = heartbeat->record.lon;
      }
    }
  }
}

lt_gps_motion_state_t lt_gps_policy_motion_state(
    const lt_gps_policy_t *policy) {
  return policy_impl_const(policy)->motion_state;
}

lt_gps_policy_metrics_t lt_gps_policy_metrics(const lt_gps_policy_t *policy) {
  return policy_impl_const(policy)->metrics;
}

void lt_gps_policy_no_fix(lt_gps_policy_t *policy) {
  lt_gps_policy_impl_t *impl = policy_impl(policy);
  impl->consecutive_moving = 0U;
  impl->candidate_stable = false;
}

void lt_gps_policy_finish(lt_gps_policy_t *policy) {
  lt_gps_policy_impl_t *impl = policy_impl(policy);
  if (impl->recent_count == 0U) return;
  const lt_gps_recent_sample_t *last = &impl->recent[lt_gps_policy_recent_index(impl, impl->recent_count - 1U)];
  if (!last->jump) lt_gps_policy_persist(impl, &last->record, LT_GPS_PERSIST_SESSION_BOUNDARY);
}
