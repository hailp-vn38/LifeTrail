#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "lifetrail_batch_store.h"
#include "lifetrail_storage_queue.h"

#define CHECK(condition) do { if (!(condition)) { \
  fprintf(stderr, "%s:%d %s\n", __FILE__, __LINE__, #condition); exit(EXIT_FAILURE); \
} } while (0)

static void queue_accepted(void *context, const lt_gps_record_t *record,
                           lt_gps_persist_reason_t reason, lt_gps_motion_state_t motion) {
  (void)reason;
  lt_storage_entry_t entry = {.record = *record, .motion = motion};
  lt_storage_queue_push(context, &entry);
}

static bool uuid_bytes(void *context, uint8_t bytes[16]) {
  unsigned *counter = context;
  memset(bytes, 0, 16U);
  unsigned value = ++*counter;
  memcpy(bytes, &value, sizeof(value));
  return true;
}

static void test_queue_drops_oldest_without_blocking_the_policy(void) {
  lt_storage_entry_t entries[LT_STORAGE_QUEUE_CAPACITY];
  lt_storage_queue_t queue;
  lt_gps_policy_t policy;
  lt_storage_queue_init(&queue, entries, LT_STORAGE_QUEUE_CAPACITY, NULL, NULL, NULL);
  lt_gps_policy_init(&policy, NULL, queue_accepted, &queue);
  for (int i = 1; i <= 811; i++) {
    lt_gps_record_t record = {.ts_ms = i * INT64_C(1000),
        .lat = 10.0 + i * 0.0000126, .lon = 106.0,
        .has_speed_mps = true, .speed_mps = 1.4};
    lt_gps_policy_process(&policy, &record);
  }
  CHECK(lt_storage_queue_overflow_count(&queue) == 15U);
  lt_storage_entry_t first;
  CHECK(lt_storage_queue_pop(&queue, &first));
  CHECK(first.record.ts_ms == INT64_C(46000));
  size_t remaining = 0U;
  while (lt_storage_queue_pop(&queue, &first)) remaining++;
  CHECK(remaining == 255U);
}

static void test_buffered_batch_durability_and_idle_rotation(void) {
  char path[] = "/tmp/lifetrail-recording-XXXXXX";
  CHECK(mkdtemp(path) != NULL);
  lt_batch_store_t store;
  lt_batch_store_config_t config = {.root_path = path};
  CHECK(lt_batch_store_init(&store, &config));
  unsigned uuid_counter = 0U;
  lt_gps_batch_writer_t writer;
  lt_gps_batch_writer_init(&writer, NULL, uuid_bytes, &uuid_counter,
                           lt_batch_store_writer_sink(&store));
  for (int i = 1; i <= 15; i++) {
    CHECK(lt_batch_store_poll(&store, (uint64_t)i * 1000U, false));
    lt_gps_record_t record = {.ts_ms = INT64_C(1791158400000) + i * 1000,
        .lat = 10.0, .lon = 106.0, .fix_quality = 1U, .satellites = 8U};
    CHECK(lt_gps_batch_writer_append_with_motion(&writer, &record, LT_GPS_MOTION_MOVING));
  }
  CHECK(lt_batch_store_poll(&store, 16000U, false));
  lt_batch_store_health_t health = lt_batch_store_health(&store);
  CHECK(health.sd_append_count == 15U);
  CHECK(health.sd_flush_count <= 3U);
  CHECK(health.sd_fsync_count == 1U);
  CHECK(lt_gps_batch_writer_poll(&writer, INT64_C(1791158461000), LT_GPS_MOTION_MOVING));
  lt_batch_store_ready_t ready;
  CHECK(lt_batch_store_next_ready(&store, &ready));
  CHECK(ready.record_count == 15U);
  CHECK(lt_batch_store_close(&store));
  /* Reboot validates the exact flushed bytes and manifest. */
  lt_batch_store_t recovered;
  CHECK(lt_batch_store_init(&recovered, &config));
  CHECK(lt_batch_store_next_ready(&recovered, &ready));
  CHECK(ready.record_count == 15U);
}

static void test_eight_hour_recording_reduces_actual_fsyncs(void) {
  char path[] = "/tmp/lifetrail-stationary-writes-XXXXXX";
  CHECK(mkdtemp(path) != NULL);
  lt_batch_store_t store;
  lt_batch_store_config_t config = {.root_path = path};
  CHECK(lt_batch_store_init(&store, &config));
  unsigned uuid_counter = 0U;
  lt_gps_batch_writer_t writer;
  lt_gps_batch_writer_init(&writer, NULL, uuid_bytes, &uuid_counter,
                           lt_batch_store_writer_sink(&store));
  lt_storage_entry_t entries[LT_STORAGE_QUEUE_CAPACITY];
  lt_storage_queue_t queue;
  lt_storage_queue_init(&queue, entries, LT_STORAGE_QUEUE_CAPACITY, NULL, NULL, NULL);
  lt_gps_policy_t policy;
  lt_gps_policy_init(&policy, NULL, queue_accepted, &queue);
  for (int i = 0; i < 28800; i++) {
    CHECK(lt_batch_store_poll(&store, (uint64_t)i * 1000U, false));
    lt_gps_record_t record = {.ts_ms = INT64_C(1791158400000) + i * 1000,
        .lat = 10.0, .lon = 106.0, .speed_mps = 0.0, .has_speed_mps = true,
        .fix_quality = 1U, .satellites = 8U};
    lt_gps_policy_process(&policy, &record);
    lt_storage_entry_t entry;
    while (lt_storage_queue_pop(&queue, &entry)) {
      CHECK(lt_gps_batch_writer_append_with_motion(&writer, &entry.record, entry.motion));
    }
    CHECK(lt_gps_batch_writer_poll(&writer, record.ts_ms, lt_gps_policy_motion_state(&policy)));
  }
  lt_gps_policy_finish(&policy);
  lt_storage_entry_t last;
  while (lt_storage_queue_pop(&queue, &last))
    CHECK(lt_gps_batch_writer_append_with_motion(&writer, &last.record, last.motion));
  CHECK(lt_gps_batch_writer_finish(&writer));
  lt_batch_store_health_t health = lt_batch_store_health(&store);
  CHECK(health.sd_append_count * 20U < 28800U);
  CHECK(health.sd_fsync_count * 20U < 28800U);
  CHECK(lt_storage_queue_overflow_count(&queue) == 0U);
  printf("stationary 8h: epochs=28800 append=%llu flush=%llu fsync=%llu\n",
         (unsigned long long)health.sd_append_count, (unsigned long long)health.sd_flush_count,
         (unsigned long long)health.sd_fsync_count);
}

static void test_delayed_backlog_rotates_by_record_time_until_drained(void) {
  char path[] = "/tmp/lifetrail-backlog-XXXXXX";
  CHECK(mkdtemp(path) != NULL);
  lt_batch_store_t store;
  lt_batch_store_config_t config = {.root_path = path};
  CHECK(lt_batch_store_init(&store, &config));
  unsigned uuid_counter = 0U;
  lt_gps_batch_writer_t writer;
  lt_gps_batch_writer_init(&writer, NULL, uuid_bytes, &uuid_counter,
                           lt_batch_store_writer_sink(&store));
  /* SD becomes available ten minutes later. Queued records keep their own
     age: 120 seconds of one-Hz evidence forms two batches, not 120. */
  for (int second = 0; second < 120; second++) {
    lt_gps_record_t record = {.ts_ms = INT64_C(1791158400000) + second * 1000,
        .lat = 10.0, .lon = 106.0, .fix_quality = 1U, .satellites = 8U};
    CHECK(lt_gps_batch_writer_append_with_motion(&writer, &record, LT_GPS_MOTION_MOVING));
  }
  CHECK(lt_gps_batch_writer_poll(&writer, INT64_C(1791159000000), LT_GPS_MOTION_MOVING));
  CHECK(uuid_counter == 2U);
  CHECK(lt_batch_store_health(&store).sd_append_count == 120U);
  CHECK(lt_batch_store_health(&store).sd_fsync_count < 10U);
  CHECK(lt_batch_store_close(&store));
}

int main(void) {
  test_delayed_backlog_rotates_by_record_time_until_drained();
  test_queue_drops_oldest_without_blocking_the_policy();
  test_buffered_batch_durability_and_idle_rotation();
  test_eight_hour_recording_reduces_actual_fsyncs();
  return EXIT_SUCCESS;
}
