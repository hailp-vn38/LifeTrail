#pragma once

#include <stdbool.h>
#include "lifetrail_batch_store.h"
#include "lifetrail_gps_policy.h"

typedef struct {
  lt_gps_policy_metrics_t gps;
  lt_batch_store_health_t storage;
  uint64_t storage_queue_overflow_count;
  lt_gps_motion_state_t motion;
} lt_recorder_health_t;

/* Starts SD recording and GPS UART acquisition. GPIOs come from Kconfig. */
bool lt_recorder_start(void);
/* Stops acquisition, drains the queue, rotates and fsyncs before returning. */
bool lt_recorder_shutdown(void);
lt_recorder_health_t lt_recorder_health(void);
