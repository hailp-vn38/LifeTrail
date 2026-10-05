#include "lifetrail_sync.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

#include "esp_http_client.h"

static bool set_headers(esp_http_client_handle_t client, const lt_sync_request_t *request,
                        const char *authorization) {
  char byte_length[24];
  char record_count[16];
  return snprintf(byte_length, sizeof(byte_length), "%" PRIu64, request->batch->byte_length) > 0 &&
      snprintf(record_count, sizeof(record_count), "%" PRIu32, request->batch->record_count) > 0 &&
      esp_http_client_set_header(client, "Content-Type", "application/x-ndjson") == ESP_OK &&
      esp_http_client_set_header(client, "Authorization", authorization) == ESP_OK &&
      esp_http_client_set_header(client, "X-LifeTrail-Batch-Id", request->batch->batch_id) == ESP_OK &&
      esp_http_client_set_header(client, "X-LifeTrail-Schema", "gps/1") == ESP_OK &&
      esp_http_client_set_header(client, "X-LifeTrail-Content-SHA256", request->batch->sha256) == ESP_OK &&
      esp_http_client_set_header(client, "X-LifeTrail-Byte-Length", byte_length) == ESP_OK &&
      esp_http_client_set_header(client, "X-LifeTrail-Record-Count", record_count) == ESP_OK;
}

bool lt_sync_esp_http_upload(void *context, const lt_sync_request_t *request,
                             lt_sync_response_t *response) {
  lt_sync_esp_http_t *result = context;
  char url[256];
  char authorization[80];
  FILE *file;
  esp_http_client_handle_t client = NULL;
  esp_http_client_config_t config;
  char bytes[1024];
  size_t read;
  char *retry_after = NULL;
  if (result == NULL || request == NULL || response == NULL ||
      snprintf(url, sizeof(url), "%s%s", request->api_url, LT_SYNC_UPLOAD_PATH) < 0 ||
      snprintf(authorization, sizeof(authorization), "Bearer %s", request->device_token) < 0)
    return false;
  memset(result, 0, sizeof(*result));
  file = fopen(request->body_path, "rb");
  if (file == NULL) return false;
  config = (esp_http_client_config_t){.url = url, .method = HTTP_METHOD_POST, .timeout_ms = 15000};
  client = esp_http_client_init(&config);
  if (client == NULL || !set_headers(client, request, authorization) ||
      esp_http_client_open(client, (int)request->batch->byte_length) != ESP_OK) goto failed;
  while ((read = fread(bytes, 1U, sizeof(bytes), file)) > 0U)
    if (esp_http_client_write(client, bytes, (int)read) != (int)read) goto failed;
  if (ferror(file) || esp_http_client_fetch_headers(client) < 0) goto failed;
  response->transport = LT_SYNC_TRANSPORT_OK;
  response->status_code = esp_http_client_get_status_code(client);
  int body_length = esp_http_client_read_response(client, result->response_body,
                                                   sizeof(result->response_body) - 1U);
  if (body_length < 0) goto failed;
  result->response_body[body_length] = '\0';
  if (esp_http_client_get_header(client, "Retry-After", &retry_after) == ESP_OK && retry_after != NULL)
    (void)snprintf(result->retry_after, sizeof(result->retry_after), "%s", retry_after);
  response->body = result->response_body;
  response->retry_after = result->retry_after[0] == '\0' ? NULL : result->retry_after;
  esp_http_client_cleanup(client); fclose(file); return true;
failed:
  if (client != NULL) esp_http_client_cleanup(client);
  fclose(file);
  return false;
}
