#include "lifetrail_provisioning.h"

#include <stdio.h>
#include <string.h>

static bool copy_value(char *destination, size_t capacity, const char *value) {
  size_t length;
  if (destination == NULL || value == NULL) return false;
  length = strlen(value);
  if (length == 0U || length >= capacity) return false;
  memcpy(destination, value, length + 1U);
  return true;
}

bool lt_provisioning_init(lt_provisioning_t *provisioning,
                          lt_provisioning_backend_t backend) {
  if (provisioning == NULL || backend.load == NULL || backend.save == NULL ||
      backend.reset == NULL) return false;
  memset(provisioning, 0, sizeof(*provisioning));
  provisioning->backend = backend;
  provisioning->loaded = backend.load(backend.context, &provisioning->config);
  return true;
}

bool lt_provisioning_set_wifi(lt_provisioning_t *provisioning, const char *ssid,
                              const char *password) {
  lt_provisioning_config_t candidate;
  if (provisioning == NULL) return false;
  candidate = provisioning->config;
  if (!copy_value(candidate.wifi_ssid, sizeof(candidate.wifi_ssid), ssid) ||
      !copy_value(candidate.wifi_password, sizeof(candidate.wifi_password), password)) return false;
  if (provisioning->backend.save == NULL ||
      !provisioning->backend.save(provisioning->backend.context, &candidate)) return false;
  provisioning->config = candidate;
  provisioning->loaded = true;
  return true;
}

bool lt_provisioning_set_api_url(lt_provisioning_t *provisioning,
                                 const char *api_url) {
  lt_provisioning_config_t candidate;
  if (provisioning == NULL) return false;
  candidate = provisioning->config;
  if (!copy_value(candidate.api_url, sizeof(candidate.api_url), api_url) ||
      provisioning->backend.save == NULL ||
      !provisioning->backend.save(provisioning->backend.context, &candidate)) return false;
  provisioning->config = candidate;
  provisioning->loaded = true;
  return true;
}

bool lt_provisioning_set_device_token(lt_provisioning_t *provisioning,
                                      const char *device_token) {
  lt_provisioning_config_t candidate;
  if (provisioning == NULL) return false;
  candidate = provisioning->config;
  if (!copy_value(candidate.device_token, sizeof(candidate.device_token), device_token) ||
      provisioning->backend.save == NULL ||
      !provisioning->backend.save(provisioning->backend.context, &candidate)) return false;
  provisioning->config = candidate;
  provisioning->loaded = true;
  return true;
}

bool lt_provisioning_factory_reset(lt_provisioning_t *provisioning) {
  if (provisioning == NULL || !provisioning->backend.reset(provisioning->backend.context))
    return false;
  memset(&provisioning->config, 0, sizeof(provisioning->config));
  provisioning->loaded = false;
  return true;
}

bool lt_provisioning_is_ready(const lt_provisioning_t *provisioning) {
  const lt_provisioning_config_t *config = lt_provisioning_config(provisioning);
  return config != NULL && config->wifi_ssid[0] != '\0' && config->api_url[0] != '\0' &&
         config->device_token[0] != '\0';
}

const lt_provisioning_config_t *lt_provisioning_config(
    const lt_provisioning_t *provisioning) {
  return provisioning == NULL ? NULL : &provisioning->config;
}

lt_provisioning_command_result_t lt_provisioning_execute_serial(
    lt_provisioning_t *provisioning, const char *command) {
  char ssid[LT_PROVISIONING_SSID_MAX + 1U];
  char password[LT_PROVISIONING_PASSWORD_MAX + 1U];
  char value[LT_PROVISIONING_API_URL_MAX + 1U];
  if (provisioning == NULL || command == NULL) return LT_PROVISIONING_COMMAND_INVALID;
  if (strcmp(command, "status") == 0) return LT_PROVISIONING_COMMAND_STATUS;
  if (strcmp(command, "factory-reset") == 0)
    return lt_provisioning_factory_reset(provisioning) ? LT_PROVISIONING_COMMAND_RESET :
                                                       LT_PROVISIONING_COMMAND_INVALID;
  if (sscanf(command, "wifi %32s %64s", ssid, password) == 2)
    return lt_provisioning_set_wifi(provisioning, ssid, password) ?
        LT_PROVISIONING_COMMAND_UPDATED : LT_PROVISIONING_COMMAND_INVALID;
  if (sscanf(command, "api %192s", value) == 1)
    return lt_provisioning_set_api_url(provisioning, value) ?
        LT_PROVISIONING_COMMAND_UPDATED : LT_PROVISIONING_COMMAND_INVALID;
  if (sscanf(command, "token %64s", value) == 1)
    return lt_provisioning_set_device_token(provisioning, value) ?
        LT_PROVISIONING_COMMAND_UPDATED : LT_PROVISIONING_COMMAND_INVALID;
  return LT_PROVISIONING_COMMAND_INVALID;
}
