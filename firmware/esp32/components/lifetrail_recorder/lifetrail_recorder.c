#include "lifetrail_recorder.h"
#include "lifetrail_recorder_internal.h"

#include "esp_random.h"
#include "esp_timer.h"
#include "esp_system.h"
#include "esp_log.h"
#include "freertos/task.h"

static lt_recorder_state_t recorder;

uint64_t lt_recorder_now_ms(void) { return (uint64_t)esp_timer_get_time() / 1000U; }
void lt_recorder_lock(void *context) {
  portENTER_CRITICAL((portMUX_TYPE *)context);
}
void lt_recorder_unlock(void *context) {
  portEXIT_CRITICAL((portMUX_TYPE *)context);
}
bool lt_recorder_is_stopping(lt_recorder_state_t *state) {
  lt_recorder_lock(&state->mux);
  bool stopping = state->stopping;
  lt_recorder_unlock(&state->mux);
  return stopping;
}
static bool random_uuid(void *context, uint8_t bytes[16]) {
  (void)context;
  esp_fill_random(bytes, 16U);
  return true;
}

static void controlled_shutdown(void) {
  if (!lt_recorder_shutdown()) ESP_LOGE("lt_recorder", "storage shutdown did not complete");
}

bool lt_recorder_store_init(lt_recorder_state_t *state) {
  lt_batch_store_config_t store_config = {
      .root_path = CONFIG_LT_SD_MOUNT_POINT "/gps",
      .flush_interval_ms = CONFIG_LT_STORAGE_FLUSH_MS,
      .fsync_interval_ms = CONFIG_LT_STORAGE_FSYNC_MS,
  };
  return lt_batch_store_init(&state->store, &store_config);
}

bool lt_recorder_start(void) {
  if (recorder.started) return false;
  esp_task_wdt_config_t watchdog = {
      .timeout_ms = CONFIG_LT_RECORDER_WATCHDOG_S * 1000U,
      .idle_core_mask = (1U << portNUM_PROCESSORS) - 1U,
      .trigger_panic = true,
  };
  esp_err_t watchdog_result = esp_task_wdt_reconfigure(&watchdog);
  if (watchdog_result == ESP_ERR_INVALID_STATE)
    watchdog_result = esp_task_wdt_init(&watchdog);
  if (watchdog_result != ESP_OK) return false;
  recorder.mux = (portMUX_TYPE)portMUX_INITIALIZER_UNLOCKED;
  recorder.events = xEventGroupCreate();
  if (recorder.events == NULL || !lt_recorder_uart_init()) return false;
  recorder.storage_mounted = lt_recorder_sd_mount();
  recorder.storage_ready = recorder.storage_mounted && lt_recorder_store_init(&recorder);
  lt_storage_queue_init(&recorder.queue, recorder.entries,
      CONFIG_LT_GPS_STORAGE_QUEUE_LEN, lt_recorder_lock, lt_recorder_unlock, &recorder.mux);
  lt_gps_batch_settings_t batches = {
      .max_age_moving_ms = CONFIG_LT_GPS_BATCH_MOVING_S * UINT64_C(1000),
      .max_age_stationary_ms = CONFIG_LT_GPS_BATCH_STATIONARY_S * UINT64_C(1000),
      .max_bytes = LT_GPS_BATCH_DEFAULT_MAX_BYTES,
  };
  lt_gps_batch_writer_init(&recorder.writer, &batches, random_uuid, NULL,
                           lt_batch_store_writer_sink(&recorder.store));
  recorder.started = true;
  if (xTaskCreate(lt_recorder_storage_task, "lt_storage", LT_RECORDER_STORAGE_STACK_BYTES,
                  &recorder, LT_RECORDER_STORAGE_PRIORITY, NULL) != pdPASS) {
    recorder.started = false;
    return false;
  }
  if (xTaskCreate(lt_recorder_gps_task, "lt_gps", LT_RECORDER_GPS_STACK_BYTES,
                  &recorder, LT_RECORDER_GPS_PRIORITY, NULL) != pdPASS) {
    lt_recorder_lock(&recorder.mux);
    recorder.stopping = true;
    lt_recorder_unlock(&recorder.mux);
    xEventGroupSetBits(recorder.events, LT_RECORDER_GPS_STOPPED);
    return false;
  }
  return esp_register_shutdown_handler(controlled_shutdown) == ESP_OK;
}

bool lt_recorder_shutdown(void) {
  if (!recorder.started) return true;
  lt_recorder_lock(&recorder.mux);
  recorder.stopping = true;
  lt_recorder_unlock(&recorder.mux);
  EventBits_t done = xEventGroupWaitBits(recorder.events,
      LT_RECORDER_GPS_STOPPED | LT_RECORDER_STORAGE_STOPPED,
      pdFALSE, pdTRUE, pdMS_TO_TICKS(LT_RECORDER_SHUTDOWN_TIMEOUT_MS));
  return (done & (LT_RECORDER_GPS_STOPPED | LT_RECORDER_STORAGE_STOPPED)) ==
      (LT_RECORDER_GPS_STOPPED | LT_RECORDER_STORAGE_STOPPED) && recorder.shutdown_ok;
}

lt_recorder_health_t lt_recorder_health(void) {
  if (!recorder.started) return (lt_recorder_health_t){0};
  lt_recorder_lock(&recorder.mux);
  lt_recorder_health_t health = {.gps = recorder.policy_metrics,
      .storage = recorder.storage_health, .motion = recorder.latest_motion};
  uint64_t latest_arrival = recorder.latest_arrival_ms;
  bool has_fix = recorder.latest_ts_ms > 0;
  lt_recorder_unlock(&recorder.mux);
  health.storage_queue_overflow_count = lt_storage_queue_overflow_count(&recorder.queue);
  uint64_t now = lt_recorder_now_ms();
  if (has_fix && now > latest_arrival + UINT64_C(1000))
    health.gps.no_fix_duration_ms += now - latest_arrival - UINT64_C(1000);
  return health;
}
