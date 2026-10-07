#pragma once

#include "lifetrail_gps_policy.h"

#define LT_GPS_POLICY_RECENT_CAPACITY 60U
#define LT_GPS_POLICY_CENTER_MAX_EPOCHS 30U
#define LT_GPS_POLICY_MOVING_MIN_EPOCHS 3U
#define LT_GPS_POLICY_FAR_MOVING_MIN_EPOCHS 2U

typedef struct {
  lt_gps_record_t record;
  bool jump;
} lt_gps_recent_sample_t;

typedef struct {
  lt_gps_policy_settings_t settings;
  lt_gps_persist_sink_t persist_sink;
  void *persist_sink_context;
  lt_gps_motion_state_t motion_state;
  uint64_t state_since_ms;
  uint64_t stable_since_ms;
  bool candidate_stable;
  uint64_t next_heartbeat_ms;
  lt_gps_policy_metrics_t metrics;
  lt_gps_recent_sample_t recent[LT_GPS_POLICY_RECENT_CAPACITY];
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
} lt_gps_policy_impl_t;

double lt_gps_policy_distance_m(const lt_gps_record_t *left,
                         const lt_gps_record_t *right);
double lt_gps_policy_center_distance_m(const lt_gps_policy_impl_t *impl,
                                const lt_gps_record_t *record);
size_t lt_gps_policy_recent_index(const lt_gps_policy_impl_t *impl, size_t offset);
void lt_gps_policy_append_recent(lt_gps_policy_impl_t *impl, const lt_gps_record_t *record,
                          bool jump);
bool lt_gps_policy_is_jump(const lt_gps_policy_impl_t *impl, const lt_gps_record_t *record);
bool lt_gps_policy_candidate_condition(const lt_gps_policy_impl_t *impl, uint64_t now_ms);
void lt_gps_policy_initialize_center(lt_gps_policy_impl_t *impl);
bool lt_gps_policy_moving_evidence(const lt_gps_policy_impl_t *impl,
                            const lt_gps_record_t *record);
void lt_gps_policy_update_moving_counter(lt_gps_policy_impl_t *impl,
                                  const lt_gps_record_t *record, bool jump);
void lt_gps_policy_persist(lt_gps_policy_impl_t *impl, const lt_gps_record_t *record,
                    lt_gps_persist_reason_t reason);
void lt_gps_policy_backfill(lt_gps_policy_impl_t *impl, uint64_t transition_ms);
bool lt_gps_policy_heading_trigger(const lt_gps_policy_impl_t *impl,
                            const lt_gps_record_t *record);
const lt_gps_recent_sample_t *lt_gps_policy_heartbeat_sample(const lt_gps_policy_impl_t *impl,
                                                uint64_t deadline_ms);
