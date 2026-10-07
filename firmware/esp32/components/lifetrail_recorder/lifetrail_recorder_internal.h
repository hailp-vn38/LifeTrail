#pragma once

#include "lifetrail_batch_store.h"
#include "lifetrail_gps.h"
#include "lifetrail_storage_queue.h"
#include "freertos/FreeRTOS.h"
#include "freertos/event_groups.h"
#include "esp_task_wdt.h"

#define LT_RECORDER_GPS_STOPPED BIT0
#define LT_RECORDER_STORAGE_STOPPED BIT1
#define LT_RECORDER_STORAGE_STACK_BYTES 8192U
#define LT_RECORDER_GPS_STACK_BYTES 4096U
#define LT_RECORDER_STORAGE_PRIORITY 4U
#define LT_RECORDER_GPS_PRIORITY 5U
#define LT_RECORDER_RETRY_MS UINT64_C(60000)
#define LT_RECORDER_UNAVAILABLE_POLL_MS 100U
#define LT_RECORDER_STORAGE_POLL_MS 10U
#define LT_RECORDER_UART_POLL_MS 200U
#define LT_RECORDER_SHUTDOWN_TIMEOUT_MS 10000U

typedef struct {
  lt_gps_collector_t collector;
  lt_gps_policy_t policy;
  lt_storage_queue_t queue;
  lt_storage_entry_t entries[CONFIG_LT_GPS_STORAGE_QUEUE_LEN];
  lt_batch_store_t store;
  lt_gps_batch_writer_t writer;
  portMUX_TYPE mux;
  EventGroupHandle_t events;
  bool stopping;
  bool shutdown_ok;
  bool started;
  bool storage_mounted;
  bool storage_ready;
  int64_t latest_ts_ms;
  uint64_t latest_arrival_ms;
  lt_gps_motion_state_t latest_motion;
  lt_gps_policy_metrics_t policy_metrics;
  lt_batch_store_health_t storage_health;
} lt_recorder_state_t;

uint64_t lt_recorder_now_ms(void);
void lt_recorder_lock(void *context);
void lt_recorder_unlock(void *context);
bool lt_recorder_is_stopping(lt_recorder_state_t *state);
bool lt_recorder_uart_init(void);
bool lt_recorder_sd_mount(void);
void lt_recorder_gps_task(void *context);
void lt_recorder_storage_task(void *context);
lt_gps_policy_settings_t lt_recorder_policy_settings(void);
bool lt_recorder_store_init(lt_recorder_state_t *state);
