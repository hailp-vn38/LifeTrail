#include "lifetrail_recorder.h"
#include "esp_log.h"

void app_main(void) {
  if (!lt_recorder_start()) ESP_LOGE("lifetrail", "GPS recorder startup failed");
}
