#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "lifetrail_gps_policy.h"

#define ASSERT_TRUE(condition)                                             \
  do {                                                                     \
    if (!(condition)) {                                                    \
      fprintf(stderr, "%s:%d assertion failed: %s\\n", __FILE__, __LINE__, \
              #condition);                                                 \
      exit(EXIT_FAILURE);                                                  \
    }                                                                      \
  } while (0)

typedef struct {
  lt_gps_record_t records[512];
  lt_gps_persist_reason_t reasons[512];
  lt_gps_motion_state_t states[512];
  size_t count;
} capture_t;

static void capture(void *context, const lt_gps_record_t *record,
                    lt_gps_persist_reason_t reason,
                    lt_gps_motion_state_t state) {
  capture_t *events = context;
  ASSERT_TRUE(events->count < 512U);
  events->records[events->count] = *record;
  events->reasons[events->count] = reason;
  events->states[events->count++] = state;
}

static lt_gps_record_t sample(int64_t second, double lat, double lon,
                              double speed, double course) {
  return (lt_gps_record_t){
      .ts_ms = second * INT64_C(1000), .lat = lat, .lon = lon,
      .speed_mps = speed, .course_deg = course,
      .has_speed_mps = true, .has_course_deg = true,
  };
}

static void feed(lt_gps_policy_t *policy, int first_second, int last_second,
                 double lat_step, double speed, double course) {
  int second;
  for (second = first_second; second <= last_second; second++) {
    lt_gps_record_t record = sample(second, 10.0 + second * lat_step, 106.0,
                                    speed, course);
    lt_gps_policy_process(policy, &record);
  }
}

static void test_stationary_uses_heartbeat_instead_of_one_hz(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 151, 0.0, 0.0, 0.0);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_STATIONARY);
  ASSERT_TRUE(events.count < 20U);
  ASSERT_TRUE(events.records[events.count - 1U].ts_ms <= INT64_C(151000));
}

static void test_heading_bypasses_moving_minimum_interval(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_record_t first = sample(1, 10.0, 106.0, 15.0, 0.0);
  lt_gps_record_t turn = sample(2, 10.0001, 106.0, 15.0, 90.0);
  lt_gps_policy_init(&policy, NULL, capture, &events);
  lt_gps_policy_process(&policy, &first);
  lt_gps_policy_process(&policy, &turn);
  ASSERT_TRUE(events.count == 2U);
  ASSERT_TRUE(events.reasons[1] == LT_GPS_PERSIST_HEADING);
}

static void test_moving_transition_backfills_recent_observations(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  size_t before;
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 151, 0.0, 0.0, 0.0);
  before = events.count;
  feed(&policy, 152, 154, 0.00001, 2.0, 0.0);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_MOVING);
  ASSERT_TRUE(events.count >= before + 3U);
  ASSERT_TRUE(events.reasons[events.count - 1U] == LT_GPS_PERSIST_STATE_TRANSITION);
}

static void test_heartbeat_resumes_after_missing_fix(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  size_t before;
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 600, 0.0, 0.0, 0.0);
  before = events.count;
  feed(&policy, 29401, 29641, 0.0, 0.0, 0.0);
  ASSERT_TRUE(events.count > before);
  for (size_t i = before; i < events.count; i++) {
    ASSERT_TRUE(events.records[i].ts_ms <= INT64_C(600000) ||
                events.records[i].ts_ms >= INT64_C(29401000));
  }
}

static void test_eight_hours_stationary_and_one_lost_heartbeat(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 28800, 0.0, 0.0, 0.0);
  ASSERT_TRUE(events.count < 300U);
  size_t heartbeats = 0U;
  int64_t previous = 0;
  for (size_t i = 0; i < events.count; i++) {
    if (events.reasons[i] != LT_GPS_PERSIST_HEARTBEAT) continue;
    heartbeats++;
    if (heartbeats == 5U) continue; /* One accepted heartbeat lost downstream. */
    if (previous != 0) ASSERT_TRUE(events.records[i].ts_ms - previous <= INT64_C(240000));
    previous = events.records[i].ts_ms;
    ASSERT_TRUE(events.records[i].lat == 10.0);
  }
  ASSERT_TRUE(heartbeats >= 238U);
  ASSERT_TRUE(lt_gps_policy_metrics(&policy).max_stationary_persist_gap_ms == UINT64_C(120000));
}

static void test_walk_distance_and_heading_wrap(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 120, 0.0000126, 1.4, 90.0);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_MOVING);
  for (size_t i = 1; i < events.count; i++) {
    ASSERT_TRUE(events.records[i].ts_ms - events.records[i - 1U].ts_ms <= 3000);
    ASSERT_TRUE(events.reasons[i] != LT_GPS_PERSIST_HEADING);
  }
  memset(&events, 0, sizeof(events));
  lt_gps_policy_init(&policy, NULL, capture, &events);
  lt_gps_record_t first = sample(1, 10.0, 106.0, 15.0, 359.0);
  lt_gps_record_t next = sample(2, 10.00013, 106.0, 15.0, 2.0);
  lt_gps_record_t third = sample(3, 10.00026, 106.0, 15.0, 2.0);
  lt_gps_policy_process(&policy, &first);
  lt_gps_policy_process(&policy, &next);
  ASSERT_TRUE(events.count == 1U);
  lt_gps_policy_process(&policy, &third);
  ASSERT_TRUE(events.count == 2U && events.reasons[1] == LT_GPS_PERSIST_DISTANCE);
}

static void test_jump_is_not_a_heartbeat_or_movement_evidence(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 270, 0.0, 0.0, 0.0);
  lt_gps_record_t jump = sample(271, 10.00072, 106.0, 40.0, 90.0);
  lt_gps_policy_process(&policy, &jump);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_STATIONARY);
  ASSERT_TRUE(events.reasons[events.count - 1U] == LT_GPS_PERSIST_HEARTBEAT);
  ASSERT_TRUE(events.records[events.count - 1U].ts_ms == INT64_C(270000));
  ASSERT_TRUE(events.records[events.count - 1U].lat == 10.0);
  feed(&policy, 272, 400, 0.0, 0.0, 0.0);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_STATIONARY);
}

static void test_short_stop_and_non_increasing_epochs(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 20, 0.0, 0.0, 0.0);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_MOVING);
  size_t before = events.count;
  lt_gps_record_t old = sample(5, 20.0, 106.0, 4.0, 180.0);
  lt_gps_policy_process(&policy, &old);
  ASSERT_TRUE(events.count == before);
}

static void test_missing_epoch_breaks_consecutive_moving_evidence(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 600, 0.0, 0.0, 0.0);
  feed(&policy, 601, 602, 0.0, 1.4, 0.0);
  lt_gps_policy_no_fix(&policy);
  feed(&policy, 603, 603, 0.0, 1.4, 0.0);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_STATIONARY);
  feed(&policy, 604, 605, 0.0, 1.4, 0.0);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_MOVING);
}

static void test_jitter_has_no_write_burst(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 600, 0.0, 0.0, 0.0);
  size_t before = events.count;
  for (int i = 601; i <= 4200; i++) {
    lt_gps_record_t record = sample(i, 10.0 + (i % 7 - 3) * 0.00001, 106.0, 0.1, i % 360);
    lt_gps_policy_process(&policy, &record);
  }
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_STATIONARY);
  ASSERT_TRUE(events.count - before <= 31U);
}

static void test_stationary_confirmation_requires_uninterrupted_candidate_condition(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 90, 0.0, 0.0, 0.0);
  feed(&policy, 91, 110, 0.0, 0.8, 0.0);
  feed(&policy, 111, 151, 0.0, 0.0, 0.0);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_CANDIDATE_STOP);
  feed(&policy, 152, 270, 0.0, 0.0, 0.0);
  ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_STATIONARY);
}

static void test_finish_preserves_latest_real_observation_once(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 600, 0.0, 0.0, 0.0);
  lt_gps_policy_no_fix(&policy);
  lt_gps_policy_finish(&policy);
  ASSERT_TRUE(events.records[events.count - 1U].ts_ms == INT64_C(600000));
  size_t before = events.count;
  lt_gps_policy_finish(&policy);
  ASSERT_TRUE(events.count == before);
}

static void test_walking_recovery_does_not_reuse_stop_evidence(void) {
  lt_gps_policy_t policy;
  capture_t events = {0};
  lt_gps_policy_init(&policy, NULL, capture, &events);
  feed(&policy, 1, 600, 0.0, 0.0, 0.0);
  for (int second = 601; second <= 620; second++) {
    lt_gps_record_t record = sample(second,
        10.0 + (second - 600) * 0.0000126, 106.0, 1.4, 0.0);
    lt_gps_policy_process(&policy, &record);
    if (second >= 603)
      ASSERT_TRUE(lt_gps_policy_motion_state(&policy) == LT_GPS_MOTION_MOVING);
  }
  ASSERT_TRUE(lt_gps_policy_metrics(&policy).state_transitions == 3U);
}

int main(void) {
  test_walking_recovery_does_not_reuse_stop_evidence();
  test_stationary_uses_heartbeat_instead_of_one_hz();
  test_heading_bypasses_moving_minimum_interval();
  test_moving_transition_backfills_recent_observations();
  test_heartbeat_resumes_after_missing_fix();
  test_eight_hours_stationary_and_one_lost_heartbeat();
  test_walk_distance_and_heading_wrap();
  test_jump_is_not_a_heartbeat_or_movement_evidence();
  test_short_stop_and_non_increasing_epochs();
  test_missing_epoch_breaks_consecutive_moving_evidence();
  test_jitter_has_no_write_burst();
  test_stationary_confirmation_requires_uninterrupted_candidate_condition();
  test_finish_preserves_latest_real_observation_once();
  return EXIT_SUCCESS;
}
