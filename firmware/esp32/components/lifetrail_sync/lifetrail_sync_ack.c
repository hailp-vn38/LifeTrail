#include "lifetrail_sync_ack.h"

#include <ctype.h>
#include <stdlib.h>
#include <string.h>

typedef struct { const char *cursor; } json_t;

static void skip_space(json_t *json) { while (isspace((unsigned char)*json->cursor)) json->cursor++; }
static bool consume(json_t *json, char expected) {
  skip_space(json);
  if (*json->cursor != expected) return false;
  json->cursor++;
  return true;
}
static bool string(json_t *json, char *value, size_t capacity) {
  const char *start;
  size_t length;
  if (!consume(json, '\"')) return false;
  start = json->cursor;
  while (*json->cursor != '\0' && *json->cursor != '\"') {
    if (*json->cursor == '\\' || (unsigned char)*json->cursor < 0x20U) return false;
    json->cursor++;
  }
  length = (size_t)(json->cursor - start);
  if (*json->cursor != '\"' || length >= capacity) return false;
  memcpy(value, start, length);
  value[length] = '\0';
  json->cursor++;
  return true;
}
static bool count(json_t *json, uint32_t *value) {
  char *end;
  unsigned long parsed;
  skip_space(json);
  if (!isdigit((unsigned char)*json->cursor)) return false;
  parsed = strtoul(json->cursor, &end, 10);
  if (end == json->cursor || parsed > UINT32_MAX) return false;
  json->cursor = end;
  *value = (uint32_t)parsed;
  return true;
}
bool lt_sync_ack_matches(const lt_batch_store_ready_t *batch, const char *body) {
  json_t json = {.cursor = body};
  char key[16];
  char batch_id[37] = {0};
  char status[16] = {0};
  uint32_t record_count = 0U;
  bool duplicate;
  bool seen_batch = false, seen_status = false, seen_count = false, seen_duplicate = false;
  if (batch == NULL || body == NULL || !consume(&json, '{')) return false;
  do {
    if (!string(&json, key, sizeof(key)) || !consume(&json, ':')) return false;
    if (strcmp(key, "batch_id") == 0) {
      if (seen_batch || !string(&json, batch_id, sizeof(batch_id))) return false;
      seen_batch = true;
    } else if (strcmp(key, "status") == 0) {
      if (seen_status || !string(&json, status, sizeof(status))) return false;
      seen_status = true;
    } else if (strcmp(key, "record_count") == 0) {
      if (seen_count || !count(&json, &record_count)) return false;
      seen_count = true;
    } else if (strcmp(key, "duplicate") == 0) {
      if (seen_duplicate) return false;
      skip_space(&json);
      if (strncmp(json.cursor, "true", 4U) == 0) { duplicate = true; json.cursor += 4; }
      else if (strncmp(json.cursor, "false", 5U) == 0) { duplicate = false; json.cursor += 5; }
      else return false;
      seen_duplicate = true;
    } else return false;
    skip_space(&json);
    if (*json.cursor == '}') break;
    if (*json.cursor++ != ',') return false;
  } while (true);
  if (!consume(&json, '}')) return false;
  skip_space(&json);
  (void)duplicate;
  return *json.cursor == '\0' && seen_batch && seen_status && seen_count && seen_duplicate &&
      strcmp(batch_id, batch->batch_id) == 0 && strcmp(status, "committed") == 0 &&
      record_count == batch->record_count;
}
