#include "lifetrail_buffered_file.h"

#include <unistd.h>

bool lt_buffered_file_open(lt_buffered_file_t *buffer, const char *path) {
  if (buffer->file != NULL) return false;
  buffer->file = fopen(path, "wb");
  if (buffer->file == NULL) return false;
  if (setvbuf(buffer->file, NULL, _IOFBF, 4096U) != 0) {
    (void)fclose(buffer->file);
    buffer->file = NULL;
    return false;
  }
  buffer->last_flush_ms = buffer->now_ms;
  buffer->last_sync_ms = buffer->now_ms;
  buffer->dirty = false;
  buffer->unsynced = false;
  return true;
}

bool lt_buffered_file_append(lt_buffered_file_t *buffer, const char *bytes, size_t length) {
  if (buffer->file == NULL || fwrite(bytes, 1U, length, buffer->file) != length) return false;
  buffer->append_count++;
  buffer->dirty = true;
  buffer->unsynced = true;
  return true;
}

bool lt_buffered_file_maintain(lt_buffered_file_t *buffer, uint64_t now_ms, bool force) {
  buffer->now_ms = now_ms;
  if (buffer->file == NULL) return true;
  bool sync_due = force || now_ms - buffer->last_sync_ms >= buffer->sync_interval_ms;
  bool flush_due = force || sync_due || now_ms - buffer->last_flush_ms >= buffer->flush_interval_ms;
  if (flush_due && buffer->dirty) {
    if (fflush(buffer->file) != 0) return false;
    buffer->flush_count++;
    buffer->last_flush_ms = now_ms;
    buffer->dirty = false;
  }
  if (sync_due && buffer->unsynced) {
    if (fsync(fileno(buffer->file)) != 0) return false;
    buffer->sync_count++;
    buffer->last_sync_ms = now_ms;
    buffer->unsynced = false;
  }
  return true;
}

bool lt_buffered_file_close(lt_buffered_file_t *buffer) {
  if (buffer->file == NULL) return true;
  /* Keep the handle for a retry when the durability operation fails. */
  if (!lt_buffered_file_maintain(buffer, buffer->now_ms, true)) return false;
  FILE *file = buffer->file;
  buffer->file = NULL;
  return fclose(file) == 0;
}
