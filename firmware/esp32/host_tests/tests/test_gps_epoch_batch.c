#include <inttypes.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "lifetrail_gps.h"
#include "lifetrail_gps_batch.h"

#define ASSERT_TRUE(condition)                                             \
  do {                                                                     \
    if (!(condition)) {                                                    \
      fprintf(stderr, "%s:%d assertion failed: %s\\n", __FILE__, __LINE__, \
              #condition);                                                 \
      exit(EXIT_FAILURE);                                                  \
    }                                                                      \
  } while (0)

#define ASSERT_EQ_INT64(expected, actual) \
  ASSERT_TRUE((int64_t)(expected) == (int64_t)(actual))
#define ASSERT_EQ_UINT(expected, actual) \
  ASSERT_TRUE((uint64_t)(expected) == (uint64_t)(actual))

static const char *fixture(const char *name) {
  static char path[256];
  static char sentence[128];
  FILE *file;
  size_t length;

  ASSERT_TRUE(snprintf(path, sizeof(path), "%s/%s", LT_GPS_TEST_FIXTURES_DIR,
                       name) > 0);
  file = fopen(path, "rb");
  ASSERT_TRUE(file != NULL);
  length = fread(sentence, 1U, sizeof(sentence) - 1U, file);
  ASSERT_TRUE(fclose(file) == 0);
  ASSERT_TRUE(length > 0U && length < sizeof(sentence));
  while (length > 0U &&
         (sentence[length - 1U] == '\n' || sentence[length - 1U] == '\r')) {
    length--;
  }
  sentence[length] = '\0';
  return sentence;
}

typedef struct {
  lt_gps_record_t records[8];
  size_t count;
} record_capture_t;

static void capture_record(void *context, const lt_gps_record_t *record) {
  record_capture_t *capture = context;
  ASSERT_TRUE(capture->count < 8U);
  capture->records[capture->count++] = *record;
}

static void initialize_collector(lt_gps_collector_t *collector,
                                 record_capture_t *capture) {
  memset(capture, 0, sizeof(*capture));
  lt_gps_collector_init(collector, capture_record, capture);
}

static void test_rmc_first_emits_fractional_navigation_epoch(void) {
  lt_gps_collector_t collector;
  record_capture_t capture;
  initialize_collector(&collector, &capture);

  ASSERT_TRUE(lt_gps_collector_ingest_nmea(&collector,
                                           fixture("rmc-123519.nmea"), 100U));
  ASSERT_EQ_UINT(0U, capture.count);
  ASSERT_TRUE(lt_gps_collector_ingest_nmea(&collector,
                                           fixture("gga-123519.nmea"), 150U));

  ASSERT_EQ_UINT(1U, capture.count);
  ASSERT_EQ_INT64(INT64_C(1791203719250), capture.records[0].ts_ms);
  ASSERT_TRUE(capture.records[0].lat > 48.1172 &&
              capture.records[0].lat < 48.1174);
  ASSERT_TRUE(capture.records[0].lon > 11.5166 &&
              capture.records[0].lon < 11.5167);
  ASSERT_TRUE(capture.records[0].has_alt_m);
  ASSERT_TRUE(capture.records[0].has_speed_mps);
  ASSERT_TRUE(capture.records[0].has_course_deg);
  ASSERT_TRUE(capture.records[0].has_hdop);
  ASSERT_EQ_UINT(1U, capture.records[0].fix_quality);
  ASSERT_EQ_UINT(8U, capture.records[0].satellites);
}

static void test_gga_first_emits_navigation_epoch(void) {
  lt_gps_collector_t collector;
  record_capture_t capture;
  initialize_collector(&collector, &capture);

  ASSERT_TRUE(lt_gps_collector_ingest_nmea(&collector,
                                           fixture("gga-123519.nmea"), 100U));
  ASSERT_EQ_UINT(0U, capture.count);
  ASSERT_TRUE(lt_gps_collector_ingest_nmea(&collector,
                                           fixture("rmc-123519.nmea"), 200U));

  ASSERT_EQ_UINT(1U, capture.count);
}

static void test_mismatched_or_expired_epochs_are_diagnostic_only(void) {
  lt_gps_collector_t collector;
  record_capture_t capture;
  initialize_collector(&collector, &capture);

  ASSERT_TRUE(lt_gps_collector_ingest_nmea(&collector,
                                           fixture("gga-123519.nmea"), 100U));
  ASSERT_TRUE(lt_gps_collector_ingest_nmea(&collector,
                                           fixture("rmc-123520.nmea"), 200U));
  lt_gps_collector_expire(&collector, 2201U);

  ASSERT_EQ_UINT(0U, capture.count);
  ASSERT_EQ_UINT(
      2U, lt_gps_collector_diagnostics(&collector).navigation_epoch_unmatched);
}

static void test_duplicate_epoch_never_emits_twice(void) {
  lt_gps_collector_t collector;
  record_capture_t capture;
  initialize_collector(&collector, &capture);

  ASSERT_TRUE(lt_gps_collector_ingest_nmea(&collector,
                                           fixture("rmc-123519.nmea"), 100U));
  ASSERT_TRUE(lt_gps_collector_ingest_nmea(&collector,
                                           fixture("gga-123519.nmea"), 150U));
  ASSERT_TRUE(
      lt_gps_collector_ingest_nmea(&collector, fixture("rmc-123519.nmea"),
                                   LT_GPS_BATCH_DEFAULT_MAX_AGE_MS + 200U));
  ASSERT_TRUE(
      lt_gps_collector_ingest_nmea(&collector, fixture("gga-123519.nmea"),
                                   LT_GPS_BATCH_DEFAULT_MAX_AGE_MS + 250U));

  ASSERT_EQ_UINT(1U, capture.count);
  ASSERT_EQ_UINT(1U, lt_gps_collector_diagnostics(&collector).duplicate_epoch);
}

static void test_invalid_checksum_and_rmc_status_never_emit(void) {
  lt_gps_collector_t collector;
  record_capture_t capture;
  initialize_collector(&collector, &capture);

  ASSERT_TRUE(!lt_gps_collector_ingest_nmea(
      &collector, fixture("rmc-invalid-checksum.nmea"), 100U));
  ASSERT_TRUE(!lt_gps_collector_ingest_nmea(
      &collector, fixture("rmc-invalid-status.nmea"), 200U));
  ASSERT_TRUE(lt_gps_collector_ingest_nmea(&collector,
                                           fixture("gga-123519.nmea"), 250U));
  lt_gps_collector_expire(&collector, 2251U);

  ASSERT_EQ_UINT(0U, capture.count);
  ASSERT_EQ_UINT(1U, lt_gps_collector_diagnostics(&collector).invalid_checksum);
  ASSERT_EQ_UINT(1U, lt_gps_collector_diagnostics(&collector).invalid_rmc);
  ASSERT_EQ_UINT(
      1U, lt_gps_collector_diagnostics(&collector).navigation_epoch_unmatched);
}

typedef struct {
  char ids[4][37];
  char lines[4][512];
  size_t line_lengths[4];
  size_t opened;
  size_t appended;
  size_t rotated;
} batch_capture_t;

static bool capture_open(void *context, const char batch_id[37]) {
  batch_capture_t *capture = context;
  ASSERT_TRUE(capture->opened < 4U);
  memcpy(capture->ids[capture->opened++], batch_id, 37U);
  return true;
}

static bool capture_line(void *context, const char *line, size_t line_length) {
  batch_capture_t *capture = context;
  ASSERT_TRUE(capture->appended < 4U);
  ASSERT_TRUE(line_length < sizeof(capture->lines[0]));
  memcpy(capture->lines[capture->appended], line, line_length);
  capture->line_lengths[capture->appended++] = line_length;
  return true;
}

static bool capture_rotate(void *context) {
  batch_capture_t *capture = context;
  capture->rotated++;
  return true;
}

static bool deterministic_uuid(void *context, uint8_t random_bytes[16]) {
  uint8_t *counter = context;
  memset(random_bytes, *counter, 16U);
  (*counter)++;
  return true;
}

static lt_gps_record_t sample_record(int64_t ts_ms) {
  lt_gps_record_t record = {
      .ts_ms = ts_ms,
      .lat = 10.781234,
      .lon = 106.692345,
      .alt_m = 12.4,
      .speed_mps = 4.2,
      .course_deg = 127.5,
      .hdop = 1.3,
      .fix_quality = 1U,
      .satellites = 8U,
      .has_alt_m = true,
      .has_speed_mps = true,
      .has_course_deg = true,
      .has_hdop = true,
  };
  return record;
}

static lt_gps_batch_writer_t initialize_writer(
    batch_capture_t *capture, uint8_t *uuid_counter,
    const lt_gps_batch_settings_t *settings) {
  lt_gps_batch_writer_t writer;
  lt_gps_batch_sink_t sink = {
      .open = capture_open,
      .append_line = capture_line,
      .rotate = capture_rotate,
      .context = capture,
  };
  memset(capture, 0, sizeof(*capture));
  *uuid_counter = 1U;
  lt_gps_batch_writer_init(&writer, settings, deterministic_uuid, uuid_counter,
                           sink);
  return writer;
}

static void test_ndjson_is_lf_terminated_and_uuidv4(void) {
  batch_capture_t capture;
  uint8_t uuid_counter;
  lt_gps_batch_writer_t writer =
      initialize_writer(&capture, &uuid_counter, NULL);
  lt_gps_record_t record = sample_record(INT64_C(1791203719250));

  ASSERT_TRUE(lt_gps_batch_writer_append(&writer, &record));
  ASSERT_EQ_UINT(1U, capture.opened);
  ASSERT_EQ_UINT(1U, capture.appended);
  ASSERT_TRUE(capture.line_lengths[0] > 1U);
  ASSERT_TRUE(capture.lines[0][capture.line_lengths[0] - 1U] == '\n');
  ASSERT_TRUE(memchr(capture.lines[0], '\r', capture.line_lengths[0]) == NULL);
  ASSERT_TRUE(strstr(capture.lines[0], "\"ts_ms\":1791203719250") != NULL);
  ASSERT_TRUE(capture.ids[0][14] == '4');
  ASSERT_TRUE(strchr("89ab", capture.ids[0][19]) != NULL);
}

static void test_rotation_happens_before_time_threshold_record(void) {
  batch_capture_t capture;
  uint8_t uuid_counter;
  lt_gps_batch_writer_t writer =
      initialize_writer(&capture, &uuid_counter, NULL);
  lt_gps_record_t first = sample_record(INT64_C(1791203719250));
  lt_gps_record_t second = sample_record(
      INT64_C(1791203719250) + (int64_t)LT_GPS_BATCH_DEFAULT_MAX_AGE_MS);

  ASSERT_TRUE(lt_gps_batch_writer_append(&writer, &first));
  ASSERT_TRUE(lt_gps_batch_writer_append(&writer, &second));

  ASSERT_EQ_UINT(2U, capture.opened);
  ASSERT_EQ_UINT(2U, capture.appended);
  ASSERT_EQ_UINT(1U, capture.rotated);
  ASSERT_TRUE(strcmp(capture.ids[0], capture.ids[1]) != 0);
}

static void test_rotation_happens_before_next_line_exceeds_byte_limit(void) {
  batch_capture_t capture;
  uint8_t uuid_counter;
  lt_gps_batch_writer_t initial_writer =
      initialize_writer(&capture, &uuid_counter, NULL);
  lt_gps_record_t first = sample_record(INT64_C(1791203719250));
  ASSERT_TRUE(lt_gps_batch_writer_append(&initial_writer, &first));

  lt_gps_batch_settings_t settings = {
      .max_age_ms = LT_GPS_BATCH_DEFAULT_MAX_AGE_MS,
      .max_bytes = capture.line_lengths[0],
  };
  lt_gps_batch_writer_t writer =
      initialize_writer(&capture, &uuid_counter, &settings);
  lt_gps_record_t second = sample_record(INT64_C(1791203720999));

  ASSERT_TRUE(lt_gps_batch_writer_append(&writer, &first));
  ASSERT_TRUE(lt_gps_batch_writer_append(&writer, &second));

  ASSERT_EQ_UINT(2U, capture.opened);
  ASSERT_EQ_UINT(2U, capture.appended);
  ASSERT_EQ_UINT(1U, capture.rotated);
  ASSERT_TRUE(capture.line_lengths[0] <= settings.max_bytes);
  ASSERT_TRUE(capture.line_lengths[1] <= settings.max_bytes);
}

static void test_batch_rejects_non_increasing_timestamp(void) {
  batch_capture_t capture;
  uint8_t uuid_counter;
  lt_gps_batch_writer_t writer =
      initialize_writer(&capture, &uuid_counter, NULL);
  lt_gps_record_t first = sample_record(INT64_C(1791203719250));
  lt_gps_record_t duplicate = sample_record(INT64_C(1791203719250));

  ASSERT_TRUE(lt_gps_batch_writer_append(&writer, &first));
  ASSERT_TRUE(!lt_gps_batch_writer_append(&writer, &duplicate));
  ASSERT_EQ_UINT(1U, capture.opened);
  ASSERT_EQ_UINT(1U, capture.appended);
}

int main(void) {
  test_rmc_first_emits_fractional_navigation_epoch();
  test_gga_first_emits_navigation_epoch();
  test_mismatched_or_expired_epochs_are_diagnostic_only();
  test_duplicate_epoch_never_emits_twice();
  test_invalid_checksum_and_rmc_status_never_emit();
  test_ndjson_is_lf_terminated_and_uuidv4();
  test_rotation_happens_before_time_threshold_record();
  test_rotation_happens_before_next_line_exceeds_byte_limit();
  test_batch_rejects_non_increasing_timestamp();
  return EXIT_SUCCESS;
}
