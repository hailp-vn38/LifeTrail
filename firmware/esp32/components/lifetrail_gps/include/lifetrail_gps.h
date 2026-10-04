#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct {
  int64_t ts_ms;
  double lat;
  double lon;
  double alt_m;
  double speed_mps;
  double course_deg;
  double hdop;
  uint8_t fix_quality;
  uint8_t satellites;
  bool has_alt_m;
  bool has_speed_mps;
  bool has_course_deg;
  bool has_hdop;
} lt_gps_record_t;

typedef struct {
  uint32_t invalid_checksum;
  uint32_t invalid_rmc;
  uint32_t navigation_epoch_unmatched;
  uint32_t duplicate_epoch;
} lt_gps_diagnostics_t;

typedef void (*lt_gps_record_sink_t)(void *context,
                                     const lt_gps_record_t *record);

typedef struct {
  union {
    uint8_t bytes[1024];
    double align_double;
    void *align_pointer;
  } state;
} lt_gps_collector_t;

void lt_gps_collector_init(lt_gps_collector_t *collector,
                           lt_gps_record_sink_t record_sink,
                           void *record_sink_context);

bool lt_gps_collector_ingest_nmea(lt_gps_collector_t *collector,
                                  const char *sentence,
                                  uint64_t arrival_monotonic_ms);

void lt_gps_collector_expire(lt_gps_collector_t *collector,
                             uint64_t now_monotonic_ms);

lt_gps_diagnostics_t lt_gps_collector_diagnostics(
    const lt_gps_collector_t *collector);
