#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "lifetrail_gps.h"

typedef enum {
  LT_GPS_MOTION_MOVING,
  LT_GPS_MOTION_CANDIDATE_STOP,
  LT_GPS_MOTION_STATIONARY,
} lt_gps_motion_state_t;

typedef enum {
  LT_GPS_PERSIST_PERIODIC,
  LT_GPS_PERSIST_DISTANCE,
  LT_GPS_PERSIST_HEADING,
  LT_GPS_PERSIST_STATE_TRANSITION,
  LT_GPS_PERSIST_TRANSITION_BACKFILL,
  LT_GPS_PERSIST_HEARTBEAT,
  LT_GPS_PERSIST_SESSION_BOUNDARY,
  LT_GPS_PERSIST_FIX_RECOVERY,
} lt_gps_persist_reason_t;

typedef struct {
  uint64_t persist_min_interval_ms;
  uint64_t moving_max_interval_ms;
  double moving_distance_m;
  double heading_degrees;
  double heading_min_speed_mps;
  uint64_t heading_min_interval_ms;
  uint64_t candidate_interval_ms;
  uint64_t stationary_interval_ms;
  uint64_t candidate_detect_window_ms;
  uint64_t stationary_confirm_ms;
  uint8_t candidate_speed_ratio_pct;
  double candidate_displacement_m;
  double moving_speed_mps;
  double stationary_speed_mps;
  double stationary_drift_absorb_m;
  double stationary_move_distance_m;
  double jump_speed_mps;
  uint64_t backfill_window_ms;
  size_t backfill_max_records;
  uint64_t evidence_max_interval_ms;
  uint64_t heartbeat_selection_window_ms;
} lt_gps_policy_settings_t;

typedef struct {
  uint64_t raw_navigation_epochs;
  uint64_t persisted_records;
  uint64_t state_transitions;
  uint64_t rejected_timestamps;
  uint64_t no_fix_duration_ms;
  uint64_t moving_persist_interval_total_ms;
  uint64_t moving_persist_intervals;
  uint64_t stationary_persist_interval_total_ms;
  uint64_t stationary_persist_intervals;
  uint64_t max_stationary_persist_gap_ms;
} lt_gps_policy_metrics_t;

/* Fixed ownership, no heap allocation; capacity checked against private state. */
#define LT_GPS_POLICY_STATE_BYTES 8192U

typedef void (*lt_gps_persist_sink_t)(void *context,
                                      const lt_gps_record_t *record,
                                      lt_gps_persist_reason_t reason,
                                      lt_gps_motion_state_t motion_state);

typedef struct {
  union {
    uint8_t bytes[LT_GPS_POLICY_STATE_BYTES];
    double align_double;
    void *align_pointer;
  } state;
} lt_gps_policy_t;

void lt_gps_policy_init(lt_gps_policy_t *policy,
                        const lt_gps_policy_settings_t *settings,
                        lt_gps_persist_sink_t persist_sink,
                        void *persist_sink_context);

void lt_gps_policy_process(lt_gps_policy_t *policy,
                           const lt_gps_record_t *record);

lt_gps_motion_state_t lt_gps_policy_motion_state(
    const lt_gps_policy_t *policy);

lt_gps_policy_settings_t lt_gps_policy_default_settings(void);
lt_gps_policy_metrics_t lt_gps_policy_metrics(const lt_gps_policy_t *policy);
/* Missing/invalid Navigation Epoch breaks consecutive movement evidence. */
void lt_gps_policy_no_fix(lt_gps_policy_t *policy);
/* At graceful shutdown, enqueue the latest real non-jump observation once. */
void lt_gps_policy_finish(lt_gps_policy_t *policy);
