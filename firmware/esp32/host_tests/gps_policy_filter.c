/* Host adapter for the real firmware policy, used by simulation/acceptance. */
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

#include "lifetrail_gps_policy.h"

static void accepted(void *context, const lt_gps_record_t *record,
                     lt_gps_persist_reason_t reason, lt_gps_motion_state_t motion) {
  (void)context;
  printf("%" PRId64 ",%u,%u\n", record->ts_ms, (unsigned)motion, (unsigned)reason);
}

int main(void) {
  lt_gps_policy_t policy;
  lt_gps_policy_init(&policy, NULL, accepted, NULL);
  char line[512];
  while (fgets(line, sizeof(line), stdin) != NULL) {
    lt_gps_record_t record = {0};
    unsigned fix, satellites;
    if (sscanf(line, "%" SCNd64 ",%lf,%lf,%lf,%lf,%lf,%lf,%u,%u",
               &record.ts_ms, &record.lat, &record.lon, &record.speed_mps,
               &record.course_deg, &record.alt_m, &record.hdop, &fix, &satellites) != 9)
      return EXIT_FAILURE;
    if (fix > UINT8_MAX || satellites > UINT8_MAX) return EXIT_FAILURE;
    record.fix_quality = (uint8_t)fix;
    record.satellites = (uint8_t)satellites;
    record.has_speed_mps = isfinite(record.speed_mps);
    record.has_course_deg = isfinite(record.course_deg);
    record.has_alt_m = isfinite(record.alt_m);
    record.has_hdop = isfinite(record.hdop);
    lt_gps_policy_process(&policy, &record);
  }
  lt_gps_policy_finish(&policy);
  lt_gps_policy_metrics_t metrics = lt_gps_policy_metrics(&policy);
  fprintf(stderr, "{\"raw_navigation_epochs\":%" PRIu64 ",\"persisted_records\":%" PRIu64
          ",\"motion_state_transition_count\":%" PRIu64 ",\"no_fix_periods_total_s\":%.3f}\n",
          metrics.raw_navigation_epochs, metrics.persisted_records, metrics.state_transitions,
          (double)metrics.no_fix_duration_ms / 1000.0);
  return ferror(stdin) || ferror(stdout) ? EXIT_FAILURE : EXIT_SUCCESS;
}
