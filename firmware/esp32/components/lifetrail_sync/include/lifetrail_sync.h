#pragma once

#include <stdbool.h>
#include <stdint.h>
#include <stdatomic.h>

#include "lifetrail_batch_store.h"

#define LT_SYNC_UPLOAD_PATH "/api/v1/device/batches"

typedef enum {
  LT_SYNC_TRANSPORT_OK,
  LT_SYNC_TRANSPORT_TIMEOUT,
  LT_SYNC_TRANSPORT_UNAVAILABLE,
} lt_sync_transport_t;

typedef struct {
  const char *api_url;
  const char *device_token;
  const char *body_path;
  const lt_batch_store_ready_t *batch;
} lt_sync_request_t;

typedef struct {
  lt_sync_transport_t transport;
  int status_code;
  const char *body;
  const char *retry_after;
} lt_sync_response_t;

typedef bool (*lt_sync_upload_t)(void *context, const lt_sync_request_t *request,
                                 lt_sync_response_t *response);
typedef uint32_t (*lt_sync_jitter_t)(void *context, uint32_t upper_bound_ms);

typedef enum { LT_SYNC_IDLE, LT_SYNC_BACKING_OFF, LT_SYNC_AUTH_BLOCKED } lt_sync_state_t;

typedef struct {
  lt_batch_store_t *store;
  const char *api_url;
  const char *device_token;
  lt_sync_upload_t upload;
  void *upload_context;
  lt_sync_jitter_t jitter;
  void *jitter_context;
} lt_sync_config_t;

typedef struct {
  lt_sync_config_t config;
  lt_sync_state_t state;
  uint64_t retry_at_ms;
  uint32_t retry_attempt;
  uint32_t protocol_error;
  atomic_flag in_flight;
} lt_sync_t;

bool lt_sync_init(lt_sync_t *sync, lt_sync_config_t config);
void lt_sync_run(lt_sync_t *sync, uint64_t now_ms);
void lt_sync_retry_auth(lt_sync_t *sync);
lt_sync_state_t lt_sync_state(const lt_sync_t *sync);

typedef struct { char response_body[512]; char retry_after[32]; } lt_sync_esp_http_t;
bool lt_sync_esp_http_upload(void *context, const lt_sync_request_t *request,
                             lt_sync_response_t *response);
