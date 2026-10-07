#pragma once

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

typedef struct {
  FILE *file;
  uint64_t now_ms;
  uint64_t last_flush_ms;
  uint64_t last_sync_ms;
  uint64_t flush_interval_ms;
  uint64_t sync_interval_ms;
  uint64_t append_count;
  uint64_t flush_count;
  uint64_t sync_count;
  bool dirty;
  bool unsynced;
} lt_buffered_file_t;

bool lt_buffered_file_open(lt_buffered_file_t *buffer, const char *path);
bool lt_buffered_file_append(lt_buffered_file_t *buffer, const char *bytes, size_t length);
bool lt_buffered_file_maintain(lt_buffered_file_t *buffer, uint64_t now_ms, bool force);
bool lt_buffered_file_close(lt_buffered_file_t *buffer);
