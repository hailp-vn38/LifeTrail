#define _POSIX_C_SOURCE 200809L
#define _DARWIN_C_SOURCE

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "lifetrail_provisioning.h"
#include "lifetrail_sync.h"

#define ASSERT_TRUE(condition) do { if (!(condition)) { \
  (void)fprintf(stderr, "%s:%d assertion failed: %s\\n", __FILE__, __LINE__, #condition); \
  exit(EXIT_FAILURE); } } while (0)

typedef struct { lt_provisioning_config_t config; bool present; } config_disk_t;
typedef struct { uint64_t free_bytes; } capacity_t;
typedef struct { const int *statuses; size_t count; size_t calls; bool timeout; const char *body; } server_t;

static bool load(void *context, lt_provisioning_config_t *config) {
  config_disk_t *disk = context;
  if (disk->present) *config = disk->config;
  return disk->present;
}
static bool save(void *context, const lt_provisioning_config_t *config) {
  config_disk_t *disk = context; disk->config = *config; disk->present = true; return true;
}
static bool reset(void *context) { ((config_disk_t *)context)->present = false; return true; }
static bool free_space(void *context, uint64_t *free_bytes) { *free_bytes = ((capacity_t *)context)->free_bytes; return true; }
static bool uuid(void *context, uint8_t bytes[16]) { memset(bytes, *(uint8_t *)context, 16U); return true; }

static lt_gps_record_t record(void) {
  return (lt_gps_record_t){.ts_ms = 1000, .lat = 10.7, .lon = 106.6,
      .fix_quality = 1U, .satellites = 8U};
}
static void temp_dir(char path[128]) { strcpy(path, "/tmp/lifetrail-sync-XXXXXX"); ASSERT_TRUE(mkdtemp(path) != NULL); }
static lt_batch_store_t store_with_ready(const char *path) {
  capacity_t *capacity = malloc(sizeof(*capacity));
  uint8_t *id = malloc(sizeof(*id));
  lt_batch_store_t store;
  lt_gps_batch_writer_t writer;
  lt_gps_record_t gps_record = record();
  lt_batch_store_config_t config = {.root_path = path, .free_space = free_space,
      .free_space_context = capacity};
  *capacity = (capacity_t){.free_bytes = LT_BATCH_STORE_RESUME_BYTES + 1U}; *id = 1U;
  ASSERT_TRUE(lt_batch_store_init(&store, &config));
  lt_gps_batch_writer_init(&writer, NULL, uuid, id, lt_batch_store_writer_sink(&store));
  ASSERT_TRUE(lt_gps_batch_writer_append(&writer, &gps_record));
  ASSERT_TRUE(lt_batch_store_writer_sink(&store).rotate(&store));
  return store;
}
static bool upload(void *context, const lt_sync_request_t *request, lt_sync_response_t *response) {
  server_t *server = context;
  FILE *body = fopen(request->body_path, "rb");
  char bytes[1024]; size_t length;
  ASSERT_TRUE(body != NULL); length = fread(bytes, 1U, sizeof(bytes), body); ASSERT_TRUE(fclose(body) == 0);
  ASSERT_TRUE(length == request->batch->byte_length);
  ASSERT_TRUE(strcmp(request->api_url, "http://192.168.1.2:8080") == 0);
  ASSERT_TRUE(strcmp(request->device_token, "lt_dev_test") == 0);
  server->calls++;
  if (server->timeout) return false;
  *response = (lt_sync_response_t){.transport = LT_SYNC_TRANSPORT_OK,
      .status_code = server->statuses[server->calls - 1U],
      .body = server->body == NULL ? "{\"batch_id\":\"01010101-0101-4101-8101-010101010101\",\"status\":\"committed\",\"record_count\":1,\"duplicate\":false}" : server->body};
  return true;
}
static lt_sync_t sync_for(lt_batch_store_t *store, server_t *server) {
  lt_sync_t sync;
  ASSERT_TRUE(lt_sync_init(&sync, (lt_sync_config_t){.store = store,
      .api_url = "http://192.168.1.2:8080", .device_token = "lt_dev_test",
      .upload = upload, .upload_context = server}));
  return sync;
}
static void test_provisioning_persists_and_factory_reset_is_explicit(void) {
  config_disk_t disk = {0}; lt_provisioning_t provisioning;
  lt_provisioning_backend_t backend = {.load = load, .save = save, .reset = reset, .context = &disk};
  ASSERT_TRUE(lt_provisioning_init(&provisioning, backend));
  ASSERT_TRUE(lt_provisioning_execute_serial(&provisioning, "wifi lan secret") == LT_PROVISIONING_COMMAND_UPDATED);
  ASSERT_TRUE(lt_provisioning_execute_serial(&provisioning, "api http://192.168.1.2:8080") == LT_PROVISIONING_COMMAND_UPDATED);
  ASSERT_TRUE(lt_provisioning_execute_serial(&provisioning, "token lt_dev_test") == LT_PROVISIONING_COMMAND_UPDATED);
  ASSERT_TRUE(lt_provisioning_config(&provisioning)->wifi_ssid[0] != '\0');
  ASSERT_TRUE(lt_provisioning_config(&provisioning)->api_url[0] != '\0');
  ASSERT_TRUE(lt_provisioning_config(&provisioning)->device_token[0] != '\0');
  ASSERT_TRUE(lt_provisioning_is_ready(&provisioning));
  ASSERT_TRUE(lt_provisioning_execute_serial(&provisioning, "factory-reset") == LT_PROVISIONING_COMMAND_RESET);
  ASSERT_TRUE(!disk.present && !lt_provisioning_is_ready(&provisioning));
}
static void test_commit_and_replay_ack_move_ready_to_acked(void) {
  char path[128]; int statuses[] = {200}; server_t server = {.statuses = statuses, .count = 1U};
  lt_batch_store_ready_t ready; temp_dir(path); lt_batch_store_t store = store_with_ready(path); lt_sync_t sync = sync_for(&store, &server);
  lt_sync_run(&sync, 0U); ASSERT_TRUE(server.calls == 1U); ASSERT_TRUE(sync.protocol_error == 0U); ASSERT_TRUE(!lt_batch_store_next_ready(&store, &ready));
}
static void test_timeout_after_commit_keeps_ready_for_replay(void) {
  char path[128]; int statuses[] = {200, 200}; server_t server = {.statuses = statuses, .count = 2U, .timeout = true};
  lt_batch_store_ready_t ready; temp_dir(path); lt_batch_store_t store = store_with_ready(path); lt_sync_t sync = sync_for(&store, &server);
  lt_sync_run(&sync, 0U); ASSERT_TRUE(lt_sync_state(&sync) == LT_SYNC_BACKING_OFF); ASSERT_TRUE(lt_batch_store_next_ready(&store, &ready));
  server.timeout = false; lt_sync_run(&sync, 2000U); ASSERT_TRUE(server.calls == 2U); ASSERT_TRUE(!lt_batch_store_next_ready(&store, &ready));
}
static void test_auth_blocks_without_erasing_ready(void) {
  char path[128]; int statuses[] = {401}; server_t server = {.statuses = statuses, .count = 1U};
  lt_batch_store_ready_t ready; temp_dir(path); lt_batch_store_t store = store_with_ready(path); lt_sync_t sync = sync_for(&store, &server);
  lt_sync_run(&sync, 0U); ASSERT_TRUE(lt_sync_state(&sync) == LT_SYNC_AUTH_BLOCKED); ASSERT_TRUE(lt_batch_store_next_ready(&store, &ready));
}
static void test_mismatched_ack_keeps_ready(void) {
  char path[128]; int statuses[] = {200}; server_t server = {.statuses = statuses, .count = 1U,
      .body = "{\"batch_id\":\"01010101-0101-4101-8101-010101010101\",\"status\":\"committed\",\"record_count\":2,\"duplicate\":false}"};
  lt_batch_store_ready_t ready; temp_dir(path); lt_batch_store_t store = store_with_ready(path); lt_sync_t sync = sync_for(&store, &server);
  lt_sync_run(&sync, 0U); ASSERT_TRUE(sync.protocol_error == 1U); ASSERT_TRUE(lt_batch_store_next_ready(&store, &ready));
}
static void test_client_rejection_quarantines_one_batch(void) {
  char path[128]; int statuses[] = {422}; server_t server = {.statuses = statuses, .count = 1U};
  lt_batch_store_ready_t ready; temp_dir(path); lt_batch_store_t store = store_with_ready(path); lt_sync_t sync = sync_for(&store, &server);
  lt_sync_run(&sync, 0U); ASSERT_TRUE(!lt_batch_store_next_ready(&store, &ready));
  ASSERT_TRUE(lt_batch_store_health(&store).quarantined == 1U);
}
int main(void) {
  test_provisioning_persists_and_factory_reset_is_explicit();
  test_commit_and_replay_ack_move_ready_to_acked();
  test_timeout_after_commit_keeps_ready_for_replay();
  test_auth_blocks_without_erasing_ready();
  test_mismatched_ack_keeps_ready();
  test_client_rejection_quarantines_one_batch();
  return 0;
}
