#include "lifetrail_recorder_internal.h"

#include <inttypes.h>
#include "driver/spi_common.h"
#include "driver/sdspi_host.h"
#include "esp_vfs_fat.h"
#include "esp_log.h"
#include "freertos/task.h"

static const char *const tag = "lt_storage";

bool lt_recorder_sd_mount(void) {
  static bool bus_ready;
  sdmmc_host_t host = SDSPI_HOST_DEFAULT();
  spi_bus_config_t bus = {
      .mosi_io_num = CONFIG_LT_SD_MOSI_GPIO, .miso_io_num = CONFIG_LT_SD_MISO_GPIO,
      .sclk_io_num = CONFIG_LT_SD_CLK_GPIO, .quadwp_io_num = -1, .quadhd_io_num = -1,
      .max_transfer_sz = 4096,
  };
  if (!bus_ready) {
    if (spi_bus_initialize(host.slot, &bus, SPI_DMA_CH_AUTO) != ESP_OK) return false;
    bus_ready = true;
  }
  sdspi_device_config_t slot = SDSPI_DEVICE_CONFIG_DEFAULT();
  slot.gpio_cs = CONFIG_LT_SD_CS_GPIO;
  slot.host_id = host.slot;
  esp_vfs_fat_sdmmc_mount_config_t mount = {
      .format_if_mount_failed = false, .max_files = 8,
      .allocation_unit_size = 16 * 1024,
  };
  sdmmc_card_t *card;
  return esp_vfs_fat_sdspi_mount(CONFIG_LT_SD_MOUNT_POINT, &host, &slot, &mount, &card) == ESP_OK;
}

void lt_recorder_storage_task(void *context) {
  lt_recorder_state_t *state = context;
  ESP_ERROR_CHECK(esp_task_wdt_add(NULL));
  lt_storage_entry_t entry;
  bool pending = false;
  uint64_t last_warning_ms = 0U, reported_overflows = 0U, last_health_ms = 0U;
  uint64_t next_mount_attempt_ms = LT_RECORDER_RETRY_MS;
  while (true) {
    ESP_ERROR_CHECK(esp_task_wdt_reset());
    uint64_t now = lt_recorder_now_ms();
    if (!state->storage_ready) {
      if (lt_recorder_is_stopping(state) &&
          (xEventGroupGetBits(state->events) & LT_RECORDER_GPS_STOPPED)) break;
      if (now >= next_mount_attempt_ms) {
        if (!state->storage_mounted) state->storage_mounted = lt_recorder_sd_mount();
        state->storage_ready = state->storage_mounted && lt_recorder_store_init(state);
        uint64_t overflow = lt_storage_queue_overflow_count(&state->queue);
        ESP_LOGW(tag, "storage ready=%u queue overflow total=%" PRIu64,
                 (unsigned)state->storage_ready, overflow);
        next_mount_attempt_ms = now + LT_RECORDER_RETRY_MS;
      }
      vTaskDelay(pdMS_TO_TICKS(LT_RECORDER_UNAVAILABLE_POLL_MS));
      continue;
    }
    bool ok = lt_batch_store_poll(&state->store, now, false);
    if (!pending) pending = lt_storage_queue_pop(&state->queue, &entry);
    if (ok && pending) {
      ok = lt_gps_batch_writer_append_with_motion(&state->writer, &entry.record, entry.motion);
      if (ok) pending = false;
    }
    /* Prefetch before idle rotation: queued records rotate using their own
       timestamps, not the newest acquisition time after a storage outage. */
    if (!pending) pending = lt_storage_queue_pop(&state->queue, &entry);
    lt_recorder_lock(&state->mux);
    int64_t latest_ts = state->latest_ts_ms;
    uint64_t latest_arrival = state->latest_arrival_ms;
    lt_gps_motion_state_t motion = state->latest_motion;
    lt_recorder_unlock(&state->mux);
    if (!pending && latest_ts > 0) {
      /* Extrapolate time for rotation only, never create a GPS Record. */
      ok = lt_gps_batch_writer_poll(&state->writer,
          latest_ts + (int64_t)(now >= latest_arrival ? now - latest_arrival : 0U), motion) && ok;
    }
    uint64_t overflows = lt_storage_queue_overflow_count(&state->queue);
    if ((!ok || overflows > reported_overflows) &&
        (last_warning_ms == 0U || now - last_warning_ms >= LT_RECORDER_RETRY_MS)) {
      ESP_LOGW(tag, "storage ok=%u queue overflow, dropped oldest, total=%" PRIu64,
               (unsigned)ok, overflows);
      reported_overflows = overflows;
      last_warning_ms = now;
    }
    if (now - last_health_ms >= LT_RECORDER_RETRY_MS) {
      lt_batch_store_maintain(&state->store);
      lt_batch_store_health_t health = lt_batch_store_health(&state->store);
      lt_recorder_lock(&state->mux);
      state->storage_health = health;
      lt_recorder_unlock(&state->mux);
      ESP_LOGI(tag, "append=%" PRIu64 " flush=%" PRIu64 " fsync=%" PRIu64 " overflow=%" PRIu64,
               health.sd_append_count, health.sd_flush_count, health.sd_fsync_count, overflows);
      last_health_ms = now;
    }
    if (lt_recorder_is_stopping(state) &&
        (xEventGroupGetBits(state->events) & LT_RECORDER_GPS_STOPPED) && !pending) {
      if (!lt_storage_queue_pop(&state->queue, &entry)) {
        state->shutdown_ok = lt_gps_batch_writer_finish(&state->writer) &&
                              lt_batch_store_close(&state->store);
        break;
      }
      pending = true;
    }
    vTaskDelay(pdMS_TO_TICKS(LT_RECORDER_STORAGE_POLL_MS));
  }
  ESP_ERROR_CHECK(esp_task_wdt_delete(NULL));
  xEventGroupSetBits(state->events, LT_RECORDER_STORAGE_STOPPED);
  vTaskDelete(NULL);
}
