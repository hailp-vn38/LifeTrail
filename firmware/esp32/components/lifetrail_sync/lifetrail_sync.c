#include "lifetrail_sync.h"

#include <stdlib.h>
#include <string.h>

#include "lifetrail_sync_ack.h"

#define LT_SYNC_MAX_BACKOFF_MS UINT32_C(300000)
#define LT_SYNC_BASE_BACKOFF_MS UINT32_C(1000)

static uint32_t retry_after_ms(const char *value) {
  char *end;
  unsigned long seconds;
  if (value == NULL) return 0U;
  seconds = strtoul(value, &end, 10);
  if (*value == '\0' || *end != '\0' || seconds > UINT32_MAX / 1000U) return 0U;
  return (uint32_t)seconds * 1000U;
}

static void back_off(lt_sync_t *sync, uint64_t now_ms, const char *retry_after) {
  uint32_t delay = retry_after_ms(retry_after);
  uint32_t exponent = sync->retry_attempt > 8U ? 8U : sync->retry_attempt;
  uint32_t computed = LT_SYNC_BASE_BACKOFF_MS << exponent;
  if (computed > LT_SYNC_MAX_BACKOFF_MS) computed = LT_SYNC_MAX_BACKOFF_MS;
  if (delay == 0U) {
    uint32_t jitter = sync->config.jitter == NULL ? 0U :
        sync->config.jitter(sync->config.jitter_context, computed / 4U + 1U);
    delay = computed + jitter;
  }
  sync->retry_at_ms = now_ms + delay;
  sync->retry_attempt++;
  sync->state = LT_SYNC_BACKING_OFF;
}

bool lt_sync_init(lt_sync_t *sync, lt_sync_config_t config) {
  if (sync == NULL || config.store == NULL || config.api_url == NULL ||
      config.device_token == NULL || config.upload == NULL) return false;
  memset(sync, 0, sizeof(*sync));
  atomic_flag_clear(&sync->in_flight);
  sync->config = config;
  return true;
}

void lt_sync_run(lt_sync_t *sync, uint64_t now_ms) {
  lt_batch_store_ready_t batch;
  char path[LT_BATCH_STORE_READY_PATH_MAX];
  if (sync == NULL || sync->state == LT_SYNC_AUTH_BLOCKED ||
      (sync->state == LT_SYNC_BACKING_OFF && now_ms < sync->retry_at_ms)) return;
  if (atomic_flag_test_and_set(&sync->in_flight)) return;
  sync->state = LT_SYNC_IDLE;
  while (lt_batch_store_next_ready(sync->config.store, &batch)) {
    lt_sync_response_t response = {0};
    lt_sync_request_t request;
    if (!lt_batch_store_ready_path(sync->config.store, &batch, path)) goto done;
    request = (lt_sync_request_t){.api_url = sync->config.api_url,
        .device_token = sync->config.device_token, .body_path = path, .batch = &batch};
    if (!sync->config.upload(sync->config.upload_context, &request, &response) ||
        response.transport != LT_SYNC_TRANSPORT_OK) { back_off(sync, now_ms, NULL); goto done; }
    if (response.status_code == 200) {
      if (lt_sync_ack_matches(&batch, response.body) &&
          lt_batch_store_mark_acked(sync->config.store, batch.batch_id)) {
        sync->retry_attempt = 0U;
        continue;
      }
      sync->protocol_error++;
      back_off(sync, now_ms, NULL);
      goto done;
    }
    if (response.status_code == 401 || response.status_code == 403) {
      sync->state = LT_SYNC_AUTH_BLOCKED;
      goto done;
    }
    if (response.status_code == 409 || response.status_code == 413 || response.status_code == 422) {
      if (!lt_batch_store_quarantine(sync->config.store, batch.batch_id)) {
        back_off(sync, now_ms, NULL);
        goto done;
      }
      continue;
    }
    back_off(sync, now_ms, response.status_code == 429 ? response.retry_after : NULL);
    goto done;
  }
done:
  atomic_flag_clear(&sync->in_flight);
}

void lt_sync_retry_auth(lt_sync_t *sync) {
  if (sync != NULL && sync->state == LT_SYNC_AUTH_BLOCKED) sync->state = LT_SYNC_IDLE;
}

lt_sync_state_t lt_sync_state(const lt_sync_t *sync) {
  return sync == NULL ? LT_SYNC_IDLE : sync->state;
}
