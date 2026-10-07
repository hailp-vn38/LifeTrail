#include "lifetrail_gps_policy.h"

#include <math.h>
#include <string.h>

#define LT_GPS_POLICY_RECENT_CAPACITY 60U
#define LT_GPS_POLICY_EARTH_RADIUS_M 6371000.0
#define LT_GPS_POLICY_PI 3.14159265358979323846

typedef struct {
  lt_gps_record_t record;
  bool jump;
} recent_sample_t;

typedef struct {
  lt_gps_policy_settings_t settings;
  lt_gps_persist_sink_t persist_sink;
  void *persist_sink_context;
  lt_gps_motion_state_t motion_state;
  uint64_t state_since_ms;
  recent_sample_t recent[LT_GPS_POLICY_RECENT_CAPACITY];
  size_t recent_start;
  size_t recent_count;
  bool has_last_observed;
  lt_gps_record_t last_observed;
  bool has_last_persisted;
  lt_gps_record_t last_persisted;
  uint64_t last_heading_persist_ms;
  uint8_t consecutive_moving;
  bool has_stationary_center;
  double stationary_center_lat;
  double stationary_center_lon;
} policy_impl_t;

_Static_assert(sizeof(policy_impl_t) <= sizeof(lt_gps_policy_t),
               "policy state is too small");

static policy_impl_t *policy_impl(lt_gps_policy_t *policy) {
  return (policy_impl_t *)&policy->state;
}

static const policy_impl_t *policy_impl_const(const lt_gps_policy_t *policy) {
  return (const policy_impl_t *)&policy->state;
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
      .moving_speed_mps = 1.0,
      .stationary_speed_mps = 0.6,
      .stationary_drift_absorb_m = 10.0,
      .stationary_move_distance_m = 20.0,
      .jump_speed_mps = 70.0,
      .backfill_window_ms = UINT64_C(5000),
      .backfill_max_records = 6U,
  };
}

static double radians(double degrees) {
  return degrees * LT_GPS_POLICY_PI / 180.0;
}

static double distance_m(const lt_gps_record_t *left,
                         const lt_gps_record_t *right) {
  double latitude_delta = radians(right->lat - left->lat);
  double longitude_delta = radians(right->lon - left->lon);
  double latitude_left = radians(left->lat);
  double latitude_right = radians(right->lat);
  double a = sin(latitude_delta / 2.0) * sin(latitude_delta / 2.0) +
             cos(latitude_left) * cos(latitude_right) *
                 sin(longitude_delta / 2.0) * sin(longitude_delta / 2.0);
  return LT_GPS_POLICY_EARTH_RADIUS_M * 2.0 * atan2(sqrt(a), sqrt(1.0 - a));
}

static double center_distance_m(const policy_impl_t *impl,
                                const lt_gps_record_t *record) {
  lt_gps_record_t center = {.lat = impl->stationary_center_lat,
                            .lon = impl->stationary_center_lon};
  return distance_m(&center, record);
}

static size_t recent_index(const policy_impl_t *impl, size_t offset) {
  return (impl->recent_start + offset) % LT_GPS_POLICY_RECENT_CAPACITY;
}

static void append_recent(policy_impl_t *impl, const lt_gps_record_t *record,
                          bool jump) {
  size_t index;
  if (impl->recent_count < LT_GPS_POLICY_RECENT_CAPACITY) {
    index = recent_index(impl, impl->recent_count++);
  } else {
    index = impl->recent_start;
    impl->recent_start = (impl->recent_start + 1U) % LT_GPS_POLICY_RECENT_CAPACITY;
  }
  impl->recent[index] = (recent_sample_t){.record = *record, .jump = jump};
}

static bool is_jump(const policy_impl_t *impl, const lt_gps_record_t *record) {
  double elapsed_s;
  if (!impl->has_last_observed || record->ts_ms <= impl->last_observed.ts_ms) {
    return false;
  }
  elapsed_s = (double)(record->ts_ms - impl->last_observed.ts_ms) / 1000.0;
  return distance_m(&impl->last_observed, record) / elapsed_s >
         impl->settings.jump_speed_mps;
}

static void persist(policy_impl_t *impl, const lt_gps_record_t *record,
                    lt_gps_persist_reason_t reason) {
  if (impl->has_last_persisted && record->ts_ms <= impl->last_persisted.ts_ms) {
    return;
  }
  if (impl->persist_sink != NULL) {
    impl->persist_sink(impl->persist_sink_context, record, reason,
                       impl->motion_state);
  }
  impl->last_persisted = *record;
  impl->has_last_persisted = true;
  if (reason == LT_GPS_PERSIST_HEADING) {
    impl->last_heading_persist_ms = (uint64_t)record->ts_ms;
  }
}

static bool candidate_condition(const policy_impl_t *impl, uint64_t now_ms) {
  size_t offset;
  size_t valid = 0U;
  size_t slow = 0U;
  bool covers_window = false;
  uint64_t window_start = now_ms - impl->settings.candidate_detect_window_ms;

  for (offset = 0U; offset < impl->recent_count; offset++) {
    const recent_sample_t *sample = &impl->recent[recent_index(impl, offset)];
    if ((uint64_t)sample->record.ts_ms <= window_start) covers_window = true;
    if ((uint64_t)sample->record.ts_ms < window_start) continue;
    if (sample->jump || !sample->record.has_speed_mps) continue;
    valid++;
    if (sample->record.speed_mps < impl->settings.stationary_speed_mps) slow++;
  }
  if (!covers_window || valid == 0U ||
      slow * 100U < valid * impl->settings.candidate_speed_ratio_pct) {
    return false;
  }
  for (offset = 0U; offset < impl->recent_count; offset++) {
    const recent_sample_t *first = &impl->recent[recent_index(impl, offset)];
    size_t last_offset;
    if ((uint64_t)first->record.ts_ms < window_start || first->jump) continue;
    for (last_offset = impl->recent_count; last_offset > offset; last_offset--) {
      const recent_sample_t *last =
          &impl->recent[recent_index(impl, last_offset - 1U)];
      if ((uint64_t)last->record.ts_ms >= window_start && !last->jump) {
        return distance_m(&first->record, &last->record) < 10.0;
      }
    }
  }
  return false;
}

static void initialize_stationary_center(policy_impl_t *impl) {
  size_t offset;
  size_t included = 0U;
  double lat = 0.0;
  double lon = 0.0;
  size_t start = impl->recent_count > 30U ? impl->recent_count - 30U : 0U;
  for (offset = start; offset < impl->recent_count; offset++) {
    const recent_sample_t *sample = &impl->recent[recent_index(impl, offset)];
    if (sample->jump) continue;
    lat += sample->record.lat;
    lon += sample->record.lon;
    included++;
  }
  if (included > 0U) {
    impl->stationary_center_lat = lat / (double)included;
    impl->stationary_center_lon = lon / (double)included;
    impl->has_stationary_center = true;
  }
}

static bool moving_evidence(const policy_impl_t *impl,
                            const lt_gps_record_t *record) {
  bool far_enough = impl->has_stationary_center &&
                    center_distance_m(impl, record) >=
                        impl->settings.stationary_move_distance_m;
  return impl->consecutive_moving >= 3U ||
         (far_enough && impl->consecutive_moving >= 2U);
}

static void update_moving_counter(policy_impl_t *impl,
                                  const lt_gps_record_t *record, bool jump) {
  if (!jump && record->has_speed_mps &&
      record->speed_mps >= impl->settings.moving_speed_mps) {
    if (impl->consecutive_moving < UINT8_MAX) impl->consecutive_moving++;
  } else {
    impl->consecutive_moving = 0U;
  }
}

static void persist_backfill(policy_impl_t *impl, uint64_t transition_ms) {
  size_t offset;
  size_t persisted = 0U;
  uint64_t first_ms = transition_ms - impl->settings.backfill_window_ms;
  for (offset = 0U; offset < impl->recent_count &&
                   persisted < impl->settings.backfill_max_records;
       offset++) {
    const lt_gps_record_t *record =
        &impl->recent[recent_index(impl, offset)].record;
    if ((uint64_t)record->ts_ms >= first_ms &&
        (uint64_t)record->ts_ms < transition_ms &&
        (!impl->has_last_persisted || record->ts_ms > impl->last_persisted.ts_ms)) {
      persist(impl, record, LT_GPS_PERSIST_TRANSITION_BACKFILL);
      persisted++;
    }
  }
}

static bool heading_trigger(const policy_impl_t *impl,
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

static const recent_sample_t *heartbeat_sample(const policy_impl_t *impl,
                                                uint64_t deadline_ms) {
  size_t offset;
  const recent_sample_t *chosen = NULL;
  uint64_t earliest = deadline_ms - UINT64_C(5000);
  for (offset = 0U; offset < impl->recent_count; offset++) {
    const recent_sample_t *sample = &impl->recent[recent_index(impl, offset)];
    if (sample->jump || (uint64_t)sample->record.ts_ms < earliest ||
        (uint64_t)sample->record.ts_ms > deadline_ms) continue;
    chosen = sample;
  }
  return chosen;
}

void lt_gps_policy_init(lt_gps_policy_t *policy,
                        const lt_gps_policy_settings_t *settings,
                        lt_gps_persist_sink_t persist_sink,
                        void *persist_sink_context) {
  policy_impl_t *impl = policy_impl(policy);
  memset(impl, 0, sizeof(*impl));
  impl->settings = settings == NULL ? lt_gps_policy_default_settings() : *settings;
  impl->persist_sink = persist_sink;
  impl->persist_sink_context = persist_sink_context;
  impl->motion_state = LT_GPS_MOTION_MOVING;
}

void lt_gps_policy_process(lt_gps_policy_t *policy,
                           const lt_gps_record_t *record) {
  policy_impl_t *impl = policy_impl(policy);
  bool jump;
  uint64_t now_ms;
  uint64_t elapsed_since_persist;
  if (record == NULL || record->ts_ms <= 0 || !isfinite(record->lat) ||
      !isfinite(record->lon) || record->lat < -90.0 || record->lat > 90.0 ||
      record->lon < -180.0 || record->lon > 180.0 ||
      (impl->has_last_observed && record->ts_ms <= impl->last_observed.ts_ms)) {
    return;
  }
  now_ms = (uint64_t)record->ts_ms;
  jump = is_jump(impl, record);
  append_recent(impl, record, jump);
  impl->last_observed = *record;
  impl->has_last_observed = true;
  update_moving_counter(impl, record, jump);

  if (!impl->has_last_persisted) {
    persist(impl, record, LT_GPS_PERSIST_PERIODIC);
    return;
  }
  elapsed_since_persist = now_ms - (uint64_t)impl->last_persisted.ts_ms;

  if (impl->motion_state != LT_GPS_MOTION_MOVING && moving_evidence(impl, record)) {
    impl->motion_state = LT_GPS_MOTION_MOVING;
    impl->state_since_ms = now_ms;
    impl->has_stationary_center = false;
    persist_backfill(impl, now_ms);
    persist(impl, record, LT_GPS_PERSIST_STATE_TRANSITION);
    return;
  }
  if (impl->motion_state == LT_GPS_MOTION_MOVING && candidate_condition(impl, now_ms)) {
    impl->motion_state = LT_GPS_MOTION_CANDIDATE_STOP;
    impl->state_since_ms = now_ms;
    persist(impl, record, LT_GPS_PERSIST_STATE_TRANSITION);
    return;
  }
  if (impl->motion_state == LT_GPS_MOTION_CANDIDATE_STOP &&
      now_ms - impl->state_since_ms >= impl->settings.stationary_confirm_ms &&
      candidate_condition(impl, now_ms)) {
    impl->motion_state = LT_GPS_MOTION_STATIONARY;
    impl->state_since_ms = now_ms;
    initialize_stationary_center(impl);
    persist(impl, record, LT_GPS_PERSIST_STATE_TRANSITION);
    return;
  }

  if (impl->motion_state == LT_GPS_MOTION_MOVING) {
    if (heading_trigger(impl, record)) {
      persist(impl, record, LT_GPS_PERSIST_HEADING);
    } else if (elapsed_since_persist >= impl->settings.persist_min_interval_ms &&
               distance_m(&impl->last_persisted, record) >=
                   impl->settings.moving_distance_m) {
      persist(impl, record, LT_GPS_PERSIST_DISTANCE);
    } else if (elapsed_since_persist >= impl->settings.moving_max_interval_ms) {
      persist(impl, record, LT_GPS_PERSIST_PERIODIC);
    }
  } else if (impl->motion_state == LT_GPS_MOTION_CANDIDATE_STOP) {
    if (elapsed_since_persist >= impl->settings.candidate_interval_ms) {
      persist(impl, record, LT_GPS_PERSIST_PERIODIC);
    }
  } else if (elapsed_since_persist >= impl->settings.stationary_interval_ms) {
    const recent_sample_t *heartbeat = heartbeat_sample(
        impl, (uint64_t)impl->last_persisted.ts_ms + impl->settings.stationary_interval_ms);
    if (heartbeat != NULL) {
      persist(impl, &heartbeat->record, LT_GPS_PERSIST_HEARTBEAT);
      if (impl->has_stationary_center &&
          center_distance_m(impl, &heartbeat->record) <=
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
