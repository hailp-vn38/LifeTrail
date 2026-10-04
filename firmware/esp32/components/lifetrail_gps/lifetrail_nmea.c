#include "lifetrail_nmea.h"

#include <math.h>

#include "minmea.h"

static bool is_leap_year(int year) {
  return year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
}

static bool rmc_timestamp(const struct minmea_date *date,
                          const struct minmea_time *time,
                          uint32_t *second_of_day, int64_t *ts_ms) {
  static const unsigned short days_before_month[] = {
      0U, 31U, 59U, 90U, 120U, 151U, 181U, 212U, 243U, 273U, 304U, 334U};
  int year = date->year >= 80 ? 1900 + date->year : 2000 + date->year;
  int days_in_month;
  int leap_days;
  int64_t days;

  if (date->month < 1 || date->month > 12 || date->day < 1 || time->hours < 0 ||
      time->hours > 23 || time->minutes < 0 || time->minutes > 59 ||
      time->seconds < 0 || time->seconds > 59 || time->microseconds < 0 ||
      time->microseconds >= 1000000) {
    return false;
  }
  days_in_month =
      date->month == 2 && is_leap_year(year)
          ? 29
          : (date->month == 2 ? 28
                              : ((date->month == 4 || date->month == 6 ||
                                  date->month == 9 || date->month == 11)
                                     ? 30
                                     : 31));
  if (date->day > days_in_month) {
    return false;
  }
  *second_of_day =
      (uint32_t)(time->hours * 3600 + time->minutes * 60 + time->seconds);
  leap_days = (year - 1) / 4 - (year - 1) / 100 + (year - 1) / 400 - 477;
  days = (int64_t)(year - 1970) * 365 + leap_days +
         days_before_month[date->month - 1] + date->day - 1;
  if (date->month > 2 && is_leap_year(year)) {
    days++;
  }
  *ts_ms = days * INT64_C(86400000) + (int64_t)(*second_of_day) * 1000 +
           time->microseconds / 1000;
  return *ts_ms > 0;
}

static bool gga_time(const struct minmea_time *time, uint32_t *second_of_day) {
  if (time->hours < 0 || time->hours > 23 || time->minutes < 0 ||
      time->minutes > 59 || time->seconds < 0 || time->seconds > 59) {
    return false;
  }
  *second_of_day =
      (uint32_t)(time->hours * 3600 + time->minutes * 60 + time->seconds);
  return true;
}

static lt_nmea_parse_result_t parse_rmc(const char *sentence,
                                        lt_nmea_rmc_t *output) {
  struct minmea_sentence_rmc frame;
  float speed_knots;
  float course_deg;

  if (!minmea_parse_rmc(&frame, sentence) || !frame.valid) {
    return LT_NMEA_INVALID_RECORD;
  }
  output->record = (lt_gps_record_t){0};
  if (!rmc_timestamp(&frame.date, &frame.time, &output->second_of_day,
                     &output->record.ts_ms)) {
    return LT_NMEA_INVALID_RECORD;
  }
  output->record.lat = minmea_tocoord(&frame.latitude);
  output->record.lon = minmea_tocoord(&frame.longitude);
  if (!isfinite(output->record.lat) || !isfinite(output->record.lon) ||
      output->record.lat < -90.0 || output->record.lat > 90.0 ||
      output->record.lon < -180.0 || output->record.lon > 180.0) {
    return LT_NMEA_INVALID_RECORD;
  }
  speed_knots = minmea_tofloat(&frame.speed);
  if (isfinite(speed_knots)) {
    if (speed_knots < 0.0F) {
      return LT_NMEA_INVALID_RECORD;
    }
    output->record.speed_mps = (double)speed_knots * 0.514444;
    output->record.has_speed_mps = true;
  }
  course_deg = minmea_tofloat(&frame.course);
  if (isfinite(course_deg)) {
    if (course_deg < 0.0F || course_deg >= 360.0F) {
      return LT_NMEA_INVALID_RECORD;
    }
    output->record.course_deg = course_deg;
    output->record.has_course_deg = true;
  }
  return LT_NMEA_RMC;
}

static lt_nmea_parse_result_t parse_gga(const char *sentence,
                                        lt_nmea_gga_t *output) {
  struct minmea_sentence_gga frame;
  float altitude;
  float hdop;
  uint32_t second_of_day;

  if (!minmea_parse_gga(&frame, sentence) ||
      !gga_time(&frame.time, &second_of_day) || frame.fix_quality < 0 ||
      frame.fix_quality > UINT8_MAX || frame.satellites_tracked < 0 ||
      frame.satellites_tracked > UINT8_MAX) {
    return LT_NMEA_INVALID_RECORD;
  }
  *output = (lt_nmea_gga_t){.second_of_day = second_of_day,
                            .fix_quality = (uint8_t)frame.fix_quality,
                            .satellites = (uint8_t)frame.satellites_tracked};
  altitude = minmea_tofloat(&frame.altitude);
  if (isfinite(altitude)) {
    output->alt_m = altitude;
    output->has_alt_m = true;
  }
  hdop = minmea_tofloat(&frame.hdop);
  if (isfinite(hdop)) {
    if (hdop < 0.0F) {
      return LT_NMEA_INVALID_RECORD;
    }
    output->hdop = hdop;
    output->has_hdop = true;
  }
  return LT_NMEA_GGA;
}

lt_nmea_parse_result_t lt_nmea_parse(const char *sentence, lt_nmea_rmc_t *rmc,
                                     lt_nmea_gga_t *gga) {
  if (sentence == NULL || !minmea_check(sentence, true)) {
    return LT_NMEA_INVALID_CHECKSUM;
  }
  switch (minmea_sentence_id(sentence, true)) {
    case MINMEA_SENTENCE_RMC:
      return parse_rmc(sentence, rmc);
    case MINMEA_SENTENCE_GGA:
      return parse_gga(sentence, gga);
    default:
      return LT_NMEA_OTHER;
  }
}
