#include "lifetrail_recorder_internal.h"

#include <inttypes.h>
#include "driver/uart.h"
#include "esp_log.h"
#include "freertos/task.h"

static const char *const tag = "lt_gps";

bool lt_recorder_uart_init(void) {
  const uart_config_t config = {
      .baud_rate = CONFIG_LT_GPS_UART_BAUD,
      .data_bits = UART_DATA_8_BITS, .parity = UART_PARITY_DISABLE,
      .stop_bits = UART_STOP_BITS_1, .flow_ctrl = UART_HW_FLOWCTRL_DISABLE,
      .source_clk = UART_SCLK_DEFAULT,
  };
  return uart_param_config(CONFIG_LT_GPS_UART_NUM, &config) == ESP_OK &&
      uart_set_pin(CONFIG_LT_GPS_UART_NUM, UART_PIN_NO_CHANGE, CONFIG_LT_GPS_RX_GPIO,
                    UART_PIN_NO_CHANGE, UART_PIN_NO_CHANGE) == ESP_OK &&
      uart_driver_install(CONFIG_LT_GPS_UART_NUM, 2048, 0, 0, NULL, 0) == ESP_OK;
}

static void accepted(void *context, const lt_gps_record_t *record,
                     lt_gps_persist_reason_t reason, lt_gps_motion_state_t motion) {
  lt_recorder_state_t *state = context;
  lt_storage_entry_t entry = {.record = *record, .motion = motion};
  lt_storage_queue_push(&state->queue, &entry);
  ESP_LOGD(tag, "persist reason=%u", (unsigned)reason);
}

static void observation(void *context, const lt_gps_record_t *record) {
  lt_recorder_state_t *state = context;
  lt_gps_motion_state_t before = lt_gps_policy_motion_state(&state->policy);
  lt_gps_policy_process(&state->policy, record);
  lt_gps_motion_state_t after = lt_gps_policy_motion_state(&state->policy);
  if (before != after) ESP_LOGI(tag, "motion %u -> %u", (unsigned)before, (unsigned)after);
  uint64_t arrival = lt_recorder_now_ms();
  lt_gps_policy_metrics_t metrics = lt_gps_policy_metrics(&state->policy);
  lt_recorder_lock(&state->mux);
  state->latest_ts_ms = record->ts_ms;
  state->latest_arrival_ms = arrival;
  state->latest_motion = after;
  state->policy_metrics = metrics;
  lt_recorder_unlock(&state->mux);
}

void lt_recorder_gps_task(void *context) {
  lt_recorder_state_t *state = context;
  ESP_ERROR_CHECK(esp_task_wdt_add(NULL));
  /* Configurable runtime settings keep GPS evidence out of parser logic. */
  lt_gps_policy_settings_t settings = lt_recorder_policy_settings();
  lt_gps_policy_init(&state->policy, &settings, accepted, state);
  lt_gps_collector_init(&state->collector, observation, state);
  char sentence[160];
  size_t length = 0U;
  bool overflow = false;
  while (!lt_recorder_is_stopping(state)) {
    ESP_ERROR_CHECK(esp_task_wdt_reset());
    uint8_t bytes[128];
    int count = uart_read_bytes(CONFIG_LT_GPS_UART_NUM, bytes, sizeof(bytes),
                                pdMS_TO_TICKS(LT_RECORDER_UART_POLL_MS));
    uint64_t now = lt_recorder_now_ms();
    for (int i = 0; i < count; i++) {
      if (bytes[i] == '\n') {
        sentence[length] = '\0';
        if (!overflow && !lt_gps_collector_ingest_nmea(&state->collector, sentence, now))
          lt_gps_policy_no_fix(&state->policy);
        length = 0U;
        overflow = false;
      } else if (bytes[i] != '\r') {
        if (length + 1U < sizeof(sentence)) sentence[length++] = (char)bytes[i];
        else overflow = true;
      }
    }
    lt_gps_collector_expire(&state->collector, now);
    if (count < 0) ESP_LOGW(tag, "UART read failed");
  }
  lt_gps_policy_finish(&state->policy);
  ESP_ERROR_CHECK(esp_task_wdt_delete(NULL));
  xEventGroupSetBits(state->events, LT_RECORDER_GPS_STOPPED);
  vTaskDelete(NULL);
}
