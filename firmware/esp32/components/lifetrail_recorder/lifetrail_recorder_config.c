#include "lifetrail_recorder_internal.h"

lt_gps_policy_settings_t lt_recorder_policy_settings(void) {
  lt_gps_policy_settings_t settings = lt_gps_policy_default_settings();
  settings.persist_min_interval_ms = CONFIG_LT_GPS_PERSIST_MIN_INTERVAL_MS;
  settings.moving_max_interval_ms = CONFIG_LT_GPS_MOVING_MAX_INTERVAL_MS;
  settings.moving_distance_m = CONFIG_LT_GPS_DISTANCE_M;
  settings.heading_degrees = CONFIG_LT_GPS_HEADING_DEG;
  settings.heading_min_speed_mps = CONFIG_LT_GPS_HEADING_MIN_SPEED_MMPS / 1000.0;
  settings.heading_min_interval_ms = CONFIG_LT_GPS_HEADING_MIN_INTERVAL_MS;
  settings.candidate_interval_ms = CONFIG_LT_GPS_CANDIDATE_INTERVAL_MS;
  settings.stationary_interval_ms = CONFIG_LT_GPS_STATIONARY_INTERVAL_MS;
  settings.candidate_detect_window_ms = CONFIG_LT_GPS_CANDIDATE_WINDOW_MS;
  settings.stationary_confirm_ms = CONFIG_LT_GPS_STATIONARY_CONFIRM_MS;
  settings.candidate_speed_ratio_pct = CONFIG_LT_GPS_CANDIDATE_RATIO_PCT;
  settings.candidate_displacement_m = CONFIG_LT_GPS_CANDIDATE_DISPLACEMENT_M;
  settings.moving_speed_mps = CONFIG_LT_GPS_MOVING_SPEED_MMPS / 1000.0;
  settings.stationary_speed_mps = CONFIG_LT_GPS_STATIONARY_SPEED_MMPS / 1000.0;
  settings.stationary_drift_absorb_m = CONFIG_LT_GPS_DRIFT_ABSORB_M;
  settings.stationary_move_distance_m = CONFIG_LT_GPS_MOVE_DISTANCE_M;
  settings.jump_speed_mps = CONFIG_LT_GPS_JUMP_SPEED_MPS;
  settings.backfill_window_ms = CONFIG_LT_GPS_BACKFILL_WINDOW_MS;
  settings.backfill_max_records = CONFIG_LT_GPS_BACKFILL_MAX_RECORDS;
  return settings;
}
