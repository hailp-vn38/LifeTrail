#pragma once

#include <stdbool.h>
#include <stddef.h>

#define LT_PROVISIONING_SSID_MAX 32U
#define LT_PROVISIONING_PASSWORD_MAX 64U
#define LT_PROVISIONING_API_URL_MAX 192U
#define LT_PROVISIONING_TOKEN_MAX 64U

typedef struct {
  char wifi_ssid[LT_PROVISIONING_SSID_MAX + 1U];
  char wifi_password[LT_PROVISIONING_PASSWORD_MAX + 1U];
  char api_url[LT_PROVISIONING_API_URL_MAX + 1U];
  char device_token[LT_PROVISIONING_TOKEN_MAX + 1U];
} lt_provisioning_config_t;

typedef bool (*lt_provisioning_load_t)(void *context,
                                       lt_provisioning_config_t *config);
typedef bool (*lt_provisioning_save_t)(void *context,
                                       const lt_provisioning_config_t *config);
typedef bool (*lt_provisioning_reset_t)(void *context);

typedef struct {
  lt_provisioning_load_t load;
  lt_provisioning_save_t save;
  lt_provisioning_reset_t reset;
  void *context;
} lt_provisioning_backend_t;

typedef struct {
  lt_provisioning_backend_t backend;
  lt_provisioning_config_t config;
  bool loaded;
} lt_provisioning_t;

bool lt_provisioning_init(lt_provisioning_t *provisioning,
                          lt_provisioning_backend_t backend);
bool lt_provisioning_set_wifi(lt_provisioning_t *provisioning, const char *ssid,
                              const char *password);
bool lt_provisioning_set_api_url(lt_provisioning_t *provisioning,
                                 const char *api_url);
bool lt_provisioning_set_device_token(lt_provisioning_t *provisioning,
                                      const char *device_token);
bool lt_provisioning_factory_reset(lt_provisioning_t *provisioning);
bool lt_provisioning_is_ready(const lt_provisioning_t *provisioning);
const lt_provisioning_config_t *lt_provisioning_config(
    const lt_provisioning_t *provisioning);

typedef enum {
  LT_PROVISIONING_COMMAND_INVALID,
  LT_PROVISIONING_COMMAND_UPDATED,
  LT_PROVISIONING_COMMAND_STATUS,
  LT_PROVISIONING_COMMAND_RESET,
} lt_provisioning_command_result_t;

lt_provisioning_command_result_t lt_provisioning_execute_serial(
    lt_provisioning_t *provisioning, const char *command);
