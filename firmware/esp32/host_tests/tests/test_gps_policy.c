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

int main(void) {
  test_stationary_uses_heartbeat_instead_of_one_hz();
  test_heading_bypasses_moving_minimum_interval();
  test_moving_transition_backfills_recent_observations();
  return EXIT_SUCCESS;
}
