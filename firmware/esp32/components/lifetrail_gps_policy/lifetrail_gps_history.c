#include "lifetrail_gps_policy_internal.h"

#include <math.h>

#define LT_GPS_POLICY_EARTH_RADIUS_M 6371000.0
#define LT_GPS_POLICY_PI 3.14159265358979323846

static double radians(double degrees) {
  return degrees * LT_GPS_POLICY_PI / 180.0;
}

double lt_gps_policy_distance_m(const lt_gps_record_t *left,
                         const lt_gps_record_t *right) {
  double latitude_delta = radians(right->lat - left->lat);
  double longitude_delta = radians(right->lon - left->lon);
  double latitude_left = radians(left->lat);
  double latitude_right = radians(right->lat);
  double a = sin(latitude_delta / 2.0) * sin(latitude_delta / 2.0) +
             cos(latitude_left) * cos(latitude_right) *
                 sin(longitude_delta / 2.0) * sin(longitude_delta / 2.0);
  a = fmax(0.0, fmin(1.0, a));
  return LT_GPS_POLICY_EARTH_RADIUS_M * 2.0 * atan2(sqrt(a), sqrt(1.0 - a));
}

double lt_gps_policy_center_distance_m(const lt_gps_policy_impl_t *impl,
                                const lt_gps_record_t *record) {
  lt_gps_record_t center = {.lat = impl->stationary_center_lat,
                            .lon = impl->stationary_center_lon};
  return lt_gps_policy_distance_m(&center, record);
}

size_t lt_gps_policy_recent_index(const lt_gps_policy_impl_t *impl, size_t offset) {
  return (impl->recent_start + offset) % LT_GPS_POLICY_RECENT_CAPACITY;
}

void lt_gps_policy_append_recent(lt_gps_policy_impl_t *impl, const lt_gps_record_t *record,
                          bool jump) {
  size_t index;
  if (impl->recent_count < LT_GPS_POLICY_RECENT_CAPACITY) {
    index = lt_gps_policy_recent_index(impl, impl->recent_count++);
  } else {
    index = impl->recent_start;
    impl->recent_start = (impl->recent_start + 1U) % LT_GPS_POLICY_RECENT_CAPACITY;
  }
  impl->recent[index] = (lt_gps_recent_sample_t){.record = *record, .jump = jump};
}

bool lt_gps_policy_is_jump(const lt_gps_policy_impl_t *impl, const lt_gps_record_t *record) {
  double elapsed_s;
  if (!impl->has_last_observed || record->ts_ms <= impl->last_observed.ts_ms) {
    return false;
  }
  elapsed_s = (double)(record->ts_ms - impl->last_observed.ts_ms) / 1000.0;
  return lt_gps_policy_distance_m(&impl->last_observed, record) / elapsed_s >
         impl->settings.jump_speed_mps;
}

bool lt_gps_policy_candidate_condition(const lt_gps_policy_impl_t *impl, uint64_t now_ms) {
  size_t offset;
  size_t valid = 0U;
  size_t slow = 0U;
  bool covers_window = false;
  if (now_ms < impl->settings.candidate_detect_window_ms) return false;
  uint64_t window_start = now_ms - impl->settings.candidate_detect_window_ms;

  for (offset = 0U; offset < impl->recent_count; offset++) {
    const lt_gps_recent_sample_t *sample = &impl->recent[lt_gps_policy_recent_index(impl, offset)];
    if ((uint64_t)sample->record.ts_ms <= window_start) covers_window = true;
    if ((uint64_t)sample->record.ts_ms < window_start) continue;
    if (sample->jump) continue;
    valid++;
    if (sample->record.has_speed_mps &&
        sample->record.speed_mps < impl->settings.stationary_speed_mps) slow++;
  }
  if (!covers_window || valid == 0U ||
      slow * 100U < valid * impl->settings.candidate_speed_ratio_pct) {
    return false;
  }
  for (offset = 0U; offset < impl->recent_count; offset++) {
    const lt_gps_recent_sample_t *first = &impl->recent[lt_gps_policy_recent_index(impl, offset)];
    size_t last_offset;
    if ((uint64_t)first->record.ts_ms < window_start || first->jump) continue;
    for (last_offset = impl->recent_count; last_offset > offset; last_offset--) {
      const lt_gps_recent_sample_t *last =
          &impl->recent[lt_gps_policy_recent_index(impl, last_offset - 1U)];
      if ((uint64_t)last->record.ts_ms >= window_start && !last->jump) {
        return lt_gps_policy_distance_m(&first->record, &last->record) < impl->settings.candidate_displacement_m;
      }
    }
  }
  return false;
}

void lt_gps_policy_initialize_center(lt_gps_policy_impl_t *impl) {
  size_t offset;
  size_t included = 0U;
  double lat = 0.0;
  double lon = 0.0;
  for (offset = impl->recent_count;
       offset > 0U && included < LT_GPS_POLICY_CENTER_MAX_EPOCHS; offset--) {
    const lt_gps_recent_sample_t *sample = &impl->recent[lt_gps_policy_recent_index(impl, offset - 1U)];
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

bool lt_gps_policy_moving_evidence(const lt_gps_policy_impl_t *impl,
                            const lt_gps_record_t *record) {
  bool far_enough = impl->has_stationary_center &&
                    lt_gps_policy_center_distance_m(impl, record) >=
                        impl->settings.stationary_move_distance_m;
  return impl->consecutive_moving >= LT_GPS_POLICY_MOVING_MIN_EPOCHS ||
         (far_enough && impl->consecutive_moving >= LT_GPS_POLICY_FAR_MOVING_MIN_EPOCHS);
}

void lt_gps_policy_update_moving_counter(lt_gps_policy_impl_t *impl,
                                  const lt_gps_record_t *record, bool jump) {
  if (!jump && record->has_speed_mps &&
      record->speed_mps >= impl->settings.moving_speed_mps) {
    if (impl->consecutive_moving < UINT8_MAX) impl->consecutive_moving++;
  } else {
    impl->consecutive_moving = 0U;
  }
}
