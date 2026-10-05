#include "lifetrail_provisioning_nvs.h"

#include <string.h>

#include "nvs.h"

#define LT_NVS_NAMESPACE "lifetrail"

static bool read_string(nvs_handle_t handle, const char *key, char *value, size_t size) {
  return nvs_get_str(handle, key, value, &size) == ESP_OK;
}

static bool nvs_load(void *context, lt_provisioning_config_t *config) {
  nvs_handle_t handle;
  (void)context;
  memset(config, 0, sizeof(*config));
  if (nvs_open(LT_NVS_NAMESPACE, NVS_READONLY, &handle) != ESP_OK) return false;
  bool found = read_string(handle, "wifi_ssid", config->wifi_ssid, sizeof(config->wifi_ssid));
  (void)read_string(handle, "wifi_password", config->wifi_password, sizeof(config->wifi_password));
  found = read_string(handle, "api_url", config->api_url, sizeof(config->api_url)) || found;
  found = read_string(handle, "device_token", config->device_token, sizeof(config->device_token)) || found;
  nvs_close(handle);
  return found;
}

static bool nvs_save(void *context, const lt_provisioning_config_t *config) {
  nvs_handle_t handle;
  esp_err_t error;
  (void)context;
  if (nvs_open(LT_NVS_NAMESPACE, NVS_READWRITE, &handle) != ESP_OK) return false;
  error = nvs_set_str(handle, "wifi_ssid", config->wifi_ssid);
  if (error == ESP_OK) error = nvs_set_str(handle, "wifi_password", config->wifi_password);
  if (error == ESP_OK) error = nvs_set_str(handle, "api_url", config->api_url);
  if (error == ESP_OK) error = nvs_set_str(handle, "device_token", config->device_token);
  if (error == ESP_OK) error = nvs_commit(handle);
  nvs_close(handle);
  return error == ESP_OK;
}

static bool nvs_reset(void *context) {
  nvs_handle_t handle;
  esp_err_t error;
  (void)context;
  if (nvs_open(LT_NVS_NAMESPACE, NVS_READWRITE, &handle) != ESP_OK) return false;
  error = nvs_erase_all(handle);
  if (error == ESP_OK) error = nvs_commit(handle);
  nvs_close(handle);
  return error == ESP_OK;
}

lt_provisioning_backend_t lt_provisioning_nvs_backend(void) {
  return (lt_provisioning_backend_t){.load = nvs_load, .save = nvs_save,
      .reset = nvs_reset, .context = NULL};
}
