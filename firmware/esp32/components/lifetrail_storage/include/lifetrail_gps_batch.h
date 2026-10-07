#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "lifetrail_gps.h"
#include "lifetrail_gps_policy.h"

#define LT_GPS_BATCH_DEFAULT_MAX_AGE_MS UINT64_C(300000)
#define LT_GPS_BATCH_DEFAULT_MAX_BYTES ((size_t)262144)

typedef struct {
  uint64_t max_age_ms;
  uint64_t max_age_moving_ms;
  uint64_t max_age_stationary_ms;
  size_t max_bytes;
} lt_gps_batch_settings_t;

typedef bool (*lt_gps_batch_id_source_t)(void *context,
                                         uint8_t random_bytes[16]);
typedef bool (*lt_gps_batch_open_sink_t)(void *context,
                                         const char batch_id[37]);
typedef bool (*lt_gps_batch_line_sink_t)(void *context, const char *line,
                                         size_t line_length);
typedef bool (*lt_gps_batch_rotate_sink_t)(void *context);

typedef struct {
  lt_gps_batch_open_sink_t open;
  lt_gps_batch_line_sink_t append_line;
  lt_gps_batch_rotate_sink_t rotate;
  void *context;
} lt_gps_batch_sink_t;

typedef struct {
  union {
    uint8_t bytes[256];
    double align_double;
    void *align_pointer;
  } state;
} lt_gps_batch_writer_t;

void lt_gps_batch_writer_init(lt_gps_batch_writer_t *writer,
                              const lt_gps_batch_settings_t *settings,
                              lt_gps_batch_id_source_t id_source,
                              void *id_source_context,
                              lt_gps_batch_sink_t sink);

bool lt_gps_batch_writer_append(lt_gps_batch_writer_t *writer,
                                const lt_gps_record_t *record);

bool lt_gps_batch_writer_append_with_motion(
    lt_gps_batch_writer_t *writer, const lt_gps_record_t *record,
    lt_gps_motion_state_t motion_state);
