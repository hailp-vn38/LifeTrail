#pragma once

#include <stdbool.h>
#include <stdint.h>

#include "lifetrail_gps.h"

typedef enum {
  LT_NMEA_OTHER,
  LT_NMEA_RMC,
  LT_NMEA_GGA,
  LT_NMEA_INVALID_CHECKSUM,
  LT_NMEA_INVALID_RECORD,
} lt_nmea_parse_result_t;

typedef struct {
  uint32_t second_of_day;
  lt_gps_record_t record;
} lt_nmea_rmc_t;

typedef struct {
  uint32_t second_of_day;
  double alt_m;
  double hdop;
  uint8_t fix_quality;
  uint8_t satellites;
  bool has_alt_m;
  bool has_hdop;
} lt_nmea_gga_t;

lt_nmea_parse_result_t lt_nmea_parse(const char *sentence, lt_nmea_rmc_t *rmc,
                                     lt_nmea_gga_t *gga);
