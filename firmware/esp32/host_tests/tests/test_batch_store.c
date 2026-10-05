#include <stdbool.h>
#include <stdint.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

#include "lifetrail_batch_store.h"

#define ASSERT_TRUE(condition)                                             \
  do {                                                                     \
    if (!(condition)) {                                                    \
      fprintf(stderr, "%s:%d assertion failed: %s\\n", __FILE__, __LINE__, \
              #condition);                                                 \
      exit(EXIT_FAILURE);                                                  \
    }                                                                      \
  } while (0)

typedef struct {
  uint64_t free_bytes;
} capacity_capture_t;

static bool capture_free_space(void *context, uint64_t *free_bytes) {
  *free_bytes = ((capacity_capture_t *)context)->free_bytes;
  return true;
}

static bool deterministic_uuid(void *context, uint8_t random_bytes[16]) {
  memset(random_bytes, *(uint8_t *)context, 16U);
  (*(uint8_t *)context)++;
  return true;
}

static lt_gps_record_t sample_record(int64_t ts_ms) {
  return (lt_gps_record_t){
      .ts_ms = ts_ms,
      .lat = 10.781234,
      .lon = 106.692345,
      .fix_quality = 1U,
      .satellites = 8U,
  };
}

static void make_temp_directory(char path[128]) {
  strcpy(path, "/tmp/lifetrail-batch-store-XXXXXX");
  ASSERT_TRUE(mkdtemp(path) != NULL);
}

static lt_batch_store_t initialize_store(const char *path,
                                         capacity_capture_t *capacity) {
  lt_batch_store_t store;
  lt_batch_store_config_t config = {
      .root_path = path,
      .free_space = capture_free_space,
      .free_space_context = capacity,
  };
  ASSERT_TRUE(lt_batch_store_init(&store, &config));
  return store;
}

static void append_one_record(lt_batch_store_t *store, uint8_t *uuid) {
  lt_gps_batch_writer_t writer;
  lt_gps_record_t record = sample_record(1000);
  lt_gps_batch_sink_t sink = lt_batch_store_writer_sink(store);
  lt_gps_batch_writer_init(&writer, NULL, deterministic_uuid, uuid,
                           sink);
  ASSERT_TRUE(lt_gps_batch_writer_append(&writer, &record));
  ASSERT_TRUE(sink.rotate(sink.context));
}

static void batch_path(char destination[256], const char *root,
                       const char *batch_id, const char *suffix) {
  ASSERT_TRUE(snprintf(destination, 256U, "%s/%s%s", root, batch_id, suffix) > 0);
}

static void append_bytes(const char *path, const char *bytes) {
  FILE *file = fopen(path, "ab");
  ASSERT_TRUE(file != NULL);
  ASSERT_TRUE(fwrite(bytes, 1U, strlen(bytes), file) == strlen(bytes));
  ASSERT_TRUE(fclose(file) == 0);
}

static void test_closed_open_file_recovers_to_verified_ready_batch(void) {
  char path[128];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_RESUME_BYTES};
  uint8_t uuid = 1U;
  lt_batch_store_ready_t ready;
  lt_batch_store_t store;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  ASSERT_TRUE(!lt_batch_store_health(&store).recording_paused);
  append_one_record(&store, &uuid);
  ASSERT_TRUE(lt_batch_store_next_ready(&store, &ready));
  ASSERT_TRUE(ready.record_count == 1U);
  ASSERT_TRUE(ready.byte_length > 0U);
  ASSERT_TRUE(strlen(ready.sha256) == 64U);
}

static void test_hash_mismatch_is_quarantined_not_upload_eligible(void) {
  char path[128];
  char ready_path[256];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_RESUME_BYTES};
  uint8_t uuid = 2U;
  lt_batch_store_t store;
  lt_batch_store_ready_t ready;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  append_one_record(&store, &uuid);
  batch_path(ready_path, path, "02020202-0202-4202-8202-020202020202",
             ".ndjson.ready");
  append_bytes(ready_path, "x");
  store = initialize_store(path, &capacity);
  ASSERT_TRUE(!lt_batch_store_next_ready(&store, &ready));
  ASSERT_TRUE(lt_batch_store_health(&store).quarantined == 1U);
}

static void test_partial_open_tail_is_truncated_then_finalized(void) {
  char path[128];
  char ready_path[256];
  char open_path[256];
  char manifest_path[256];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_RESUME_BYTES};
  uint8_t uuid = 4U;
  lt_batch_store_t store;
  lt_batch_store_ready_t ready;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  append_one_record(&store, &uuid);
  batch_path(ready_path, path, "04040404-0404-4404-8404-040404040404", ".ndjson.ready");
  batch_path(open_path, path, "04040404-0404-4404-8404-040404040404", ".ndjson.open");
  batch_path(manifest_path, path, "04040404-0404-4404-8404-040404040404", ".manifest");
  ASSERT_TRUE(rename(ready_path, open_path) == 0);
  ASSERT_TRUE(unlink(manifest_path) == 0);
  append_bytes(open_path, "{\"ts_ms\":2000");

  store = initialize_store(path, &capacity);
  ASSERT_TRUE(lt_batch_store_next_ready(&store, &ready));
  ASSERT_TRUE(ready.record_count == 1U);
  ASSERT_TRUE(lt_batch_store_health(&store).recovered_open == 1U);
}

static void test_malformed_committed_open_line_is_quarantined(void) {
  char path[128];
  char ready_path[256];
  char open_path[256];
  char manifest_path[256];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_RESUME_BYTES};
  uint8_t uuid = 5U;
  lt_batch_store_t store;
  lt_batch_store_ready_t ready;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  append_one_record(&store, &uuid);
  batch_path(ready_path, path, "05050505-0505-4505-8505-050505050505", ".ndjson.ready");
  batch_path(open_path, path, "05050505-0505-4505-8505-050505050505", ".ndjson.open");
  batch_path(manifest_path, path, "05050505-0505-4505-8505-050505050505", ".manifest");
  ASSERT_TRUE(rename(ready_path, open_path) == 0);
  ASSERT_TRUE(unlink(manifest_path) == 0);
  append_bytes(open_path, "not-json\n");

  store = initialize_store(path, &capacity);
  ASSERT_TRUE(!lt_batch_store_next_ready(&store, &ready));
  ASSERT_TRUE(lt_batch_store_health(&store).quarantined == 1U);
}

static void test_open_with_durable_manifest_is_promoted_after_reboot(void) {
  char path[128];
  char ready_path[256];
  char open_path[256];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_RESUME_BYTES};
  uint8_t uuid = 6U;
  lt_batch_store_t store;
  lt_batch_store_ready_t ready;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  append_one_record(&store, &uuid);
  batch_path(ready_path, path, "06060606-0606-4606-8606-060606060606", ".ndjson.ready");
  batch_path(open_path, path, "06060606-0606-4606-8606-060606060606", ".ndjson.open");
  ASSERT_TRUE(rename(ready_path, open_path) == 0);

  store = initialize_store(path, &capacity);
  ASSERT_TRUE(lt_batch_store_next_ready(&store, &ready));
  ASSERT_TRUE(lt_batch_store_health(&store).recovered_open == 1U);
}

static void test_ready_without_manifest_rebuilds_only_after_verification(void) {
  char path[128];
  char manifest_path[256];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_RESUME_BYTES};
  uint8_t uuid = 7U;
  lt_batch_store_t store;
  lt_batch_store_ready_t ready;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  append_one_record(&store, &uuid);
  batch_path(manifest_path, path, "07070707-0707-4707-8707-070707070707", ".manifest");
  ASSERT_TRUE(unlink(manifest_path) == 0);

  store = initialize_store(path, &capacity);
  ASSERT_TRUE(lt_batch_store_next_ready(&store, &ready));
  ASSERT_TRUE(lt_batch_store_health(&store).rebuilt_manifest == 1U);
}

static void test_open_with_unpublished_manifest_temp_finalizes_again(void) {
  char path[128];
  char ready_path[256];
  char open_path[256];
  char manifest_path[256];
  char temp_path[256];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_RESUME_BYTES};
  uint8_t uuid = 9U;
  lt_batch_store_t store;
  lt_batch_store_ready_t ready;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  append_one_record(&store, &uuid);
  batch_path(ready_path, path, "09090909-0909-4909-8909-090909090909", ".ndjson.ready");
  batch_path(open_path, path, "09090909-0909-4909-8909-090909090909", ".ndjson.open");
  batch_path(manifest_path, path, "09090909-0909-4909-8909-090909090909", ".manifest");
  batch_path(temp_path, path, "09090909-0909-4909-8909-090909090909", ".manifest.tmp");
  ASSERT_TRUE(rename(ready_path, open_path) == 0);
  ASSERT_TRUE(rename(manifest_path, temp_path) == 0);

  store = initialize_store(path, &capacity);
  ASSERT_TRUE(lt_batch_store_next_ready(&store, &ready));
  ASSERT_TRUE(access(temp_path, F_OK) != 0);
}

static void test_orphan_manifest_is_quarantined(void) {
  char path[128];
  char ready_path[256];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_RESUME_BYTES};
  uint8_t uuid = 10U;
  lt_batch_store_t store;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  append_one_record(&store, &uuid);
  batch_path(ready_path, path, "0a0a0a0a-0a0a-4a0a-8a0a-0a0a0a0a0a0a", ".ndjson.ready");
  ASSERT_TRUE(unlink(ready_path) == 0);

  store = initialize_store(path, &capacity);
  ASSERT_TRUE(lt_batch_store_health(&store).quarantined == 1U);
}

static void test_acknowledged_batch_expires_after_retention_without_touching_ready(void) {
  char path[128];
  char acked_path[256];
  struct timespec timestamps[2];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_LOW_SPACE_BYTES};
  uint8_t uuid = 8U;
  lt_batch_store_t store;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  append_one_record(&store, &uuid);
  ASSERT_TRUE(lt_batch_store_mark_acked(
      &store, "08080808-0808-4808-8808-080808080808"));
  batch_path(acked_path, path, "08080808-0808-4808-8808-080808080808", ".ndjson.acked");
  timestamps[0].tv_sec = time(NULL) - (time_t)LT_BATCH_STORE_ACK_RETENTION_S - 1;
  timestamps[0].tv_nsec = 0L;
  timestamps[1] = timestamps[0];
  ASSERT_TRUE(utimensat(AT_FDCWD, acked_path, timestamps, 0) == 0);
  lt_batch_store_maintain(&store);
  ASSERT_TRUE(access(acked_path, F_OK) != 0);
  ASSERT_TRUE(lt_batch_store_health(&store).acked_deleted == 1U);
}

static void test_capacity_policy_keeps_ready_data_and_has_hysteresis(void) {
  char path[128];
  capacity_capture_t capacity = {.free_bytes = LT_BATCH_STORE_RESUME_BYTES + 1U};
  uint8_t uuid = 3U;
  lt_batch_store_t store;
  lt_batch_store_ready_t ready;

  make_temp_directory(path);
  store = initialize_store(path, &capacity);
  append_one_record(&store, &uuid);
  capacity.free_bytes = LT_BATCH_STORE_PAUSE_BYTES - 1U;
  lt_batch_store_maintain(&store);
  ASSERT_TRUE(lt_batch_store_health(&store).recording_paused);
  ASSERT_TRUE(lt_batch_store_next_ready(&store, &ready));
  capacity.free_bytes = LT_BATCH_STORE_RESUME_BYTES;
  lt_batch_store_maintain(&store);
  ASSERT_TRUE(lt_batch_store_health(&store).recording_paused);
  capacity.free_bytes = LT_BATCH_STORE_RESUME_BYTES + 1U;
  lt_batch_store_maintain(&store);
  ASSERT_TRUE(!lt_batch_store_health(&store).recording_paused);
}

int main(void) {
  test_closed_open_file_recovers_to_verified_ready_batch();
  test_hash_mismatch_is_quarantined_not_upload_eligible();
  test_partial_open_tail_is_truncated_then_finalized();
  test_malformed_committed_open_line_is_quarantined();
  test_open_with_durable_manifest_is_promoted_after_reboot();
  test_ready_without_manifest_rebuilds_only_after_verification();
  test_open_with_unpublished_manifest_temp_finalizes_again();
  test_orphan_manifest_is_quarantined();
  test_acknowledged_batch_expires_after_retention_without_touching_ready();
  test_capacity_policy_keeps_ready_data_and_has_hysteresis();
  return EXIT_SUCCESS;
}
