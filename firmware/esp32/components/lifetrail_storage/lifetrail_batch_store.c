#include "lifetrail_batch_store.h"

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/statvfs.h>
#include <time.h>
#include <unistd.h>

#include "lifetrail_sha256.h"

#define LT_BATCH_STORE_FILE_PATH_MAX 384U

typedef struct {
  char batch_id[37];
  uint64_t byte_length;
  uint32_t record_count;
  int64_t first_ts_ms;
  int64_t last_ts_ms;
  char sha256[65];
} manifest_t;

typedef struct {
  char root_path[LT_BATCH_STORE_PATH_MAX];
  char active_batch_id[37];
  lt_batch_store_free_space_t free_space;
  void *free_space_context;
  lt_batch_store_health_t health;
} store_impl_t;

_Static_assert(sizeof(store_impl_t) <= sizeof(lt_batch_store_t),
               "batch store state is too small");

static store_impl_t *store_impl(lt_batch_store_t *store) {
  return (store_impl_t *)&store->state;
}

static const store_impl_t *const_store_impl(const lt_batch_store_t *store) {
  return (const store_impl_t *)&store->state;
}

static bool make_path(const store_impl_t *impl, const char *batch_id,
                      const char *suffix, char path[LT_BATCH_STORE_FILE_PATH_MAX]) {
  int written = snprintf(path, LT_BATCH_STORE_FILE_PATH_MAX, "%s/%s%s",
                         impl->root_path, batch_id, suffix);
  return written > 0 && (size_t)written < LT_BATCH_STORE_FILE_PATH_MAX;
}

static bool sync_close(FILE *file) {
  return fflush(file) == 0 && fsync(fileno(file)) == 0 && fclose(file) == 0;
}

static bool sync_data_file(const char *path) {
  int descriptor = open(path, O_RDWR);
  bool synced = descriptor >= 0 && fsync(descriptor) == 0;
  return descriptor >= 0 && close(descriptor) == 0 && synced;
}

static bool ensure_directory(const char *path) {
  return mkdir(path, 0700) == 0 || errno == EEXIST;
}

static bool valid_batch_id(const char *batch_id) {
  size_t index;
  if (batch_id == NULL || strlen(batch_id) != 36U) return false;
  for (index = 0U; index < 36U; index++) {
    bool hyphen = index == 8U || index == 13U || index == 18U || index == 23U;
    bool hex = (batch_id[index] >= '0' && batch_id[index] <= '9') ||
               (batch_id[index] >= 'a' && batch_id[index] <= 'f');
    if ((hyphen && batch_id[index] != '-') || (!hyphen && !hex)) return false;
  }
  return batch_id[14] == '4' && strchr("89ab", batch_id[19]) != NULL;
}

static bool parse_optional_number(const char value[32], bool *present,
                                  double *number) {
  char *end;
  if (strcmp(value, "null") == 0) { *present = false; return true; }
  errno = 0;
  *number = strtod(value, &end);
  *present = true;
  return errno == 0 && end != value && *end == '\0' && isfinite(*number);
}

static bool parse_record_line(const char *line, int64_t *timestamp) {
  double lat, lon, alt_m, speed_mps, course_deg, hdop;
  uint32_t fix_quality, satellites;
  char alt_text[32], speed_text[32], course_text[32], hdop_text[32];
  bool has_alt, has_speed, has_course, has_hdop;
  int fields;
  fields = sscanf(line,
                  "{\"ts_ms\":%" SCNd64 ",\"lat\":%lf,\"lon\":%lf,\"alt_m\":%31[^,],"
                  "\"speed_mps\":%31[^,],\"course_deg\":%31[^,],\"fix_quality\":%" SCNu32
                  ",\"satellites\":%" SCNu32 ",\"hdop\":%31[^}]}\n",
                  timestamp, &lat, &lon, alt_text, speed_text, course_text,
                  &fix_quality, &satellites, hdop_text);
  if (fields != 9 || *timestamp <= 0 || !isfinite(lat) || !isfinite(lon) ||
      lat < -90.0 || lat > 90.0 || lon < -180.0 || lon > 180.0 ||
      !parse_optional_number(alt_text, &has_alt, &alt_m) ||
      !parse_optional_number(speed_text, &has_speed, &speed_mps) ||
      !parse_optional_number(course_text, &has_course, &course_deg) ||
      !parse_optional_number(hdop_text, &has_hdop, &hdop)) return false;
  (void)fix_quality;
  (void)satellites;
  return (!has_speed || speed_mps >= 0.0) &&
         (!has_course || (course_deg >= 0.0 && course_deg < 360.0)) &&
         (!has_hdop || hdop >= 0.0);
}

static bool inspect_data(const char *path, bool allow_partial_tail,
                         manifest_t *manifest, bool *had_partial_tail,
                         off_t *last_valid_offset) {
  FILE *file = fopen(path, "rb");
  char line[1024];
  lt_sha256_t sha;
  uint8_t digest[32];
  int64_t previous = 0;
  off_t valid_end = 0;
  bool partial = false;
  if (file == NULL) return false;
  memset(manifest, 0, sizeof(*manifest));
  lt_sha256_init(&sha);
  while (fgets(line, sizeof(line), file) != NULL) {
    size_t length = strlen(line);
    int64_t timestamp;
    if (line[length - 1U] != '\n') { partial = true; break; }
    if (length >= sizeof(line) - 1U || line[length - 2U] == '\r' ||
        line[length - 2U] != '}' || !parse_record_line(line, &timestamp) ||
        (manifest->record_count > 0U && timestamp <= previous)) {
      (void)fclose(file);
      return false;
    }
    lt_sha256_update(&sha, (const uint8_t *)line, length);
    manifest->byte_length += length;
    if (manifest->record_count == 0U) manifest->first_ts_ms = timestamp;
    manifest->last_ts_ms = timestamp;
    previous = timestamp;
    manifest->record_count++;
    valid_end += (off_t)length;
  }
  if (ferror(file) || manifest->record_count == 0U ||
      (partial && !allow_partial_tail) || fclose(file) != 0) return false;
  lt_sha256_final(&sha, digest);
  lt_sha256_hex(digest, manifest->sha256);
  *had_partial_tail = partial;
  *last_valid_offset = valid_end;
  return true;
}

static bool metadata_matches(const manifest_t *left, const manifest_t *right) {
  return strcmp(left->batch_id, right->batch_id) == 0 &&
         left->byte_length == right->byte_length &&
         left->record_count == right->record_count &&
         left->first_ts_ms == right->first_ts_ms &&
         left->last_ts_ms == right->last_ts_ms &&
         strcmp(left->sha256, right->sha256) == 0;
}

static bool read_manifest(const store_impl_t *impl, const char batch_id[37],
                          manifest_t *manifest) {
  char path[LT_BATCH_STORE_FILE_PATH_MAX];
  FILE *file;
  char schema[16];
  int fields;
  if (!make_path(impl, batch_id, ".manifest", path)) return false;
  file = fopen(path, "rb");
  if (file == NULL) return false;
  memset(manifest, 0, sizeof(*manifest));
  fields = fscanf(file,
                  "batch_id=%36[0-9a-f-]\nschema=%15[^\n]\nbyte_length=%" SCNu64
                  "\nsha256=%64[0-9a-f]\nrecord_count=%" SCNu32
                  "\nfirst_ts_ms=%" SCNd64 "\nlast_ts_ms=%" SCNd64 "\n",
                  manifest->batch_id, schema, &manifest->byte_length,
                  manifest->sha256, &manifest->record_count,
                  &manifest->first_ts_ms, &manifest->last_ts_ms);
  return fclose(file) == 0 && fields == 7 && strcmp(schema, "gps/1") == 0 &&
         valid_batch_id(manifest->batch_id) &&
         strcmp(manifest->batch_id, batch_id) == 0 &&
         strlen(manifest->sha256) == 64U && manifest->record_count > 0U;
}

static bool manifest_exists(const store_impl_t *impl, const char batch_id[37]) {
  char path[LT_BATCH_STORE_FILE_PATH_MAX];
  struct stat info;
  return make_path(impl, batch_id, ".manifest", path) && stat(path, &info) == 0;
}

static bool write_manifest(const store_impl_t *impl, const manifest_t *manifest) {
  char tmp[LT_BATCH_STORE_FILE_PATH_MAX];
  char final[LT_BATCH_STORE_FILE_PATH_MAX];
  FILE *file;
  int written;
  if (!make_path(impl, manifest->batch_id, ".manifest.tmp", tmp) ||
      !make_path(impl, manifest->batch_id, ".manifest", final)) return false;
  file = fopen(tmp, "wb");
  if (file == NULL) return false;
  written = fprintf(file,
                    "batch_id=%s\nschema=gps/1\nbyte_length=%" PRIu64
                    "\nsha256=%s\nrecord_count=%" PRIu32
                    "\nfirst_ts_ms=%" PRId64 "\nlast_ts_ms=%" PRId64 "\n",
                    manifest->batch_id, manifest->byte_length, manifest->sha256,
                    manifest->record_count, manifest->first_ts_ms, manifest->last_ts_ms);
  if (written < 0 || !sync_close(file)) return false;
  return rename(tmp, final) == 0;
}

static void quarantine(const store_impl_t *impl, const char batch_id[37]) {
  char directory[LT_BATCH_STORE_FILE_PATH_MAX];
  char source[LT_BATCH_STORE_FILE_PATH_MAX];
  char destination[LT_BATCH_STORE_FILE_PATH_MAX];
  static const char *suffixes[] = {".ndjson.open", ".ndjson.ready", ".ndjson.acked",
                                   ".manifest", ".manifest.tmp"};
  size_t index;
  if (snprintf(directory, sizeof(directory), "%s/quarantine", impl->root_path) <= 0 ||
      !ensure_directory(directory)) return;
  for (index = 0U; index < sizeof(suffixes) / sizeof(suffixes[0]); index++) {
    if (!make_path(impl, batch_id, suffixes[index], source) ||
        snprintf(destination, sizeof(destination), "%s/%s%s", directory,
                 batch_id, suffixes[index]) <= 0) continue;
    (void)rename(source, destination);
  }
}

static bool finalize_open(store_impl_t *impl, const char batch_id[37],
                          bool allow_partial_tail) {
  char open_path[LT_BATCH_STORE_FILE_PATH_MAX];
  char ready_path[LT_BATCH_STORE_FILE_PATH_MAX];
  manifest_t actual;
  manifest_t expected;
  bool partial;
  bool has_manifest;
  off_t valid_end;
  if (!make_path(impl, batch_id, ".ndjson.open", open_path) ||
      !make_path(impl, batch_id, ".ndjson.ready", ready_path)) return false;
  has_manifest = read_manifest(impl, batch_id, &expected);
  if (!has_manifest && manifest_exists(impl, batch_id)) return false;
  if (!inspect_data(open_path, allow_partial_tail && !has_manifest, &actual,
                    &partial, &valid_end)) return false;
  if (partial && (truncate(open_path, valid_end) != 0 || !sync_data_file(open_path)))
    return false;
  memcpy(actual.batch_id, batch_id, sizeof(actual.batch_id));
  if (has_manifest) {
    if (partial || !metadata_matches(&expected, &actual)) return false;
  } else if (!write_manifest(impl, &actual)) {
    return false;
  }
  return rename(open_path, ready_path) == 0;
}

static bool suffix_batch_id(const char *name, const char *suffix, char batch_id[37]) {
  size_t suffix_length = strlen(suffix);
  if (strlen(name) != 36U + suffix_length || strcmp(name + 36U, suffix) != 0) return false;
  memcpy(batch_id, name, 36U);
  batch_id[36] = '\0';
  return valid_batch_id(batch_id);
}

static void recover_open(store_impl_t *impl, const char batch_id[37]) {
  if (finalize_open(impl, batch_id, true)) impl->health.recovered_open++;
  else { quarantine(impl, batch_id); impl->health.quarantined++; }
}

static void recover_ready(store_impl_t *impl, const char batch_id[37]) {
  char path[LT_BATCH_STORE_FILE_PATH_MAX];
  manifest_t actual;
  manifest_t expected;
  bool partial;
  off_t valid_end;
  if (!make_path(impl, batch_id, ".ndjson.ready", path) ||
      !inspect_data(path, false, &actual, &partial, &valid_end)) goto bad;
  memcpy(actual.batch_id, batch_id, sizeof(actual.batch_id));
  if (read_manifest(impl, batch_id, &expected)) {
    if (metadata_matches(&actual, &expected)) return;
    goto bad;
  }
  if (manifest_exists(impl, batch_id)) goto bad;
  if (write_manifest(impl, &actual)) { impl->health.rebuilt_manifest++; return; }
bad:
  quarantine(impl, batch_id);
  impl->health.quarantined++;
}

static bool data_file_exists(const store_impl_t *impl, const char batch_id[37]) {
  char path[LT_BATCH_STORE_FILE_PATH_MAX];
  struct stat info;
  static const char *suffixes[] = {".ndjson.open", ".ndjson.ready", ".ndjson.acked"};
  size_t index;
  for (index = 0U; index < sizeof(suffixes) / sizeof(suffixes[0]); index++)
    if (make_path(impl, batch_id, suffixes[index], path) && stat(path, &info) == 0)
      return true;
  return false;
}

static void recover_orphan_manifest(store_impl_t *impl, const char batch_id[37]) {
  if (!data_file_exists(impl, batch_id)) {
    quarantine(impl, batch_id);
    impl->health.quarantined++;
  }
}

static bool refresh_free_space(store_impl_t *impl) {
  struct statvfs info;
  uint64_t free_bytes = 0U;
  bool available;
  if (impl->free_space != NULL) {
    available = impl->free_space(impl->free_space_context, &free_bytes);
  } else {
    available = statvfs(impl->root_path, &info) == 0;
    if (available) free_bytes = (uint64_t)info.f_bavail * (uint64_t)info.f_frsize;
  }
  if (!available) return false;
  impl->health.free_bytes = free_bytes;
  impl->health.low_space = free_bytes < LT_BATCH_STORE_LOW_SPACE_BYTES;
  if (free_bytes < LT_BATCH_STORE_PAUSE_BYTES) impl->health.recording_paused = true;
  if (impl->health.recording_paused && free_bytes > LT_BATCH_STORE_RESUME_BYTES)
    impl->health.recording_paused = false;
  return true;
}

static bool open_sink(void *context, const char batch_id[37]) {
  store_impl_t *impl = store_impl(context);
  char path[LT_BATCH_STORE_FILE_PATH_MAX];
  FILE *file;
  if (impl->health.recording_paused || !valid_batch_id(batch_id) ||
      !make_path(impl, batch_id, ".ndjson.open", path)) return false;
  file = fopen(path, "wb");
  if (file == NULL || !sync_close(file)) return false;
  memcpy(impl->active_batch_id, batch_id, sizeof(impl->active_batch_id));
  return true;
}

static bool append_sink(void *context, const char *line, size_t line_length) {
  store_impl_t *impl = store_impl(context);
  char path[LT_BATCH_STORE_FILE_PATH_MAX];
  FILE *file;
  if (impl->health.recording_paused || impl->active_batch_id[0] == '\0' ||
      line == NULL || line_length == 0U ||
      !make_path(impl, impl->active_batch_id, ".ndjson.open", path)) return false;
  file = fopen(path, "ab");
  if (file == NULL) return false;
  return fwrite(line, 1U, line_length, file) == line_length && sync_close(file);
}

static bool rotate_sink(void *context) {
  store_impl_t *impl = store_impl(context);
  bool finalized;
  if (impl->active_batch_id[0] == '\0') return false;
  finalized = finalize_open(impl, impl->active_batch_id, false);
  if (!finalized) { quarantine(impl, impl->active_batch_id); impl->health.quarantined++; }
  memset(impl->active_batch_id, 0, sizeof(impl->active_batch_id));
  return finalized;
}

bool lt_batch_store_init(lt_batch_store_t *store,
                         const lt_batch_store_config_t *config) {
  store_impl_t *impl;
  DIR *directory;
  struct dirent *entry;
  char batch_id[37];
  char quarantine_path[LT_BATCH_STORE_FILE_PATH_MAX];
  if (store == NULL || config == NULL || config->root_path == NULL ||
      strlen(config->root_path) == 0U || strlen(config->root_path) >= LT_BATCH_STORE_PATH_MAX)
    return false;
  impl = store_impl(store);
  memset(impl, 0, sizeof(*impl));
  memcpy(impl->root_path, config->root_path, strlen(config->root_path) + 1U);
  impl->free_space = config->free_space;
  impl->free_space_context = config->free_space_context;
  if (!ensure_directory(impl->root_path) ||
      snprintf(quarantine_path, sizeof(quarantine_path), "%s/quarantine", impl->root_path) <= 0 ||
      !ensure_directory(quarantine_path)) return false;
  directory = opendir(impl->root_path);
  if (directory == NULL) return false;
  while ((entry = readdir(directory)) != NULL)
    if (suffix_batch_id(entry->d_name, ".ndjson.open", batch_id)) recover_open(impl, batch_id);
  rewinddir(directory);
  while ((entry = readdir(directory)) != NULL)
    if (suffix_batch_id(entry->d_name, ".ndjson.ready", batch_id)) recover_ready(impl, batch_id);
  rewinddir(directory);
  while ((entry = readdir(directory)) != NULL)
    if (suffix_batch_id(entry->d_name, ".manifest", batch_id))
      recover_orphan_manifest(impl, batch_id);
  rewinddir(directory);
  while ((entry = readdir(directory)) != NULL)
    if (suffix_batch_id(entry->d_name, ".manifest.tmp", batch_id)) {
      quarantine(impl, batch_id);
      impl->health.quarantined++;
    }
  (void)closedir(directory);
  lt_batch_store_maintain(store);
  return true;
}

lt_gps_batch_sink_t lt_batch_store_writer_sink(lt_batch_store_t *store) {
  return (lt_gps_batch_sink_t){.open = open_sink, .append_line = append_sink,
                               .rotate = rotate_sink, .context = store};
}

bool lt_batch_store_mark_acked(lt_batch_store_t *store, const char batch_id[37]) {
  store_impl_t *impl;
  char ready[LT_BATCH_STORE_FILE_PATH_MAX];
  char acked[LT_BATCH_STORE_FILE_PATH_MAX];
  manifest_t manifest;
  if (store == NULL || !valid_batch_id(batch_id)) return false;
  impl = store_impl(store);
  if (!read_manifest(impl, batch_id, &manifest) ||
      !make_path(impl, batch_id, ".ndjson.ready", ready) ||
      !make_path(impl, batch_id, ".ndjson.acked", acked)) return false;
  return rename(ready, acked) == 0;
}

bool lt_batch_store_next_ready(lt_batch_store_t *store, lt_batch_store_ready_t *ready) {
  store_impl_t *impl;
  DIR *directory;
  struct dirent *entry;
  char batch_id[37];
  char path[LT_BATCH_STORE_FILE_PATH_MAX];
  manifest_t candidate;
  manifest_t actual;
  int64_t earliest = 0;
  bool found = false;
  if (store == NULL || ready == NULL) return false;
  impl = store_impl(store);
  directory = opendir(impl->root_path);
  if (directory == NULL) return false;
  while ((entry = readdir(directory)) != NULL) {
    bool partial;
    off_t valid_end;
    if (!suffix_batch_id(entry->d_name, ".ndjson.ready", batch_id) ||
        !read_manifest(impl, batch_id, &candidate) ||
        !make_path(impl, batch_id, ".ndjson.ready", path) ||
        !inspect_data(path, false, &actual, &partial, &valid_end)) {
      continue;
    }
    memcpy(actual.batch_id, batch_id, sizeof(actual.batch_id));
    if (!metadata_matches(&actual, &candidate)) {
      quarantine(impl, batch_id);
      impl->health.quarantined++;
      continue;
    }
    if (!found || candidate.first_ts_ms < earliest ||
        (candidate.first_ts_ms == earliest && strcmp(batch_id, ready->batch_id) < 0)) {
      memcpy(ready->batch_id, batch_id, sizeof(ready->batch_id));
      ready->byte_length = candidate.byte_length;
      ready->record_count = candidate.record_count;
      memcpy(ready->sha256, candidate.sha256, sizeof(ready->sha256));
      earliest = candidate.first_ts_ms;
      found = true;
    }
  }
  (void)closedir(directory);
  return found;
}

static void delete_expired_acked(store_impl_t *impl) {
  DIR *directory = opendir(impl->root_path);
  struct dirent *entry;
  time_t now = time(NULL);
  if (directory == NULL || now == (time_t)-1) return;
  while ((entry = readdir(directory)) != NULL) {
    char batch_id[37];
    char data_path[LT_BATCH_STORE_FILE_PATH_MAX];
    char manifest_path[LT_BATCH_STORE_FILE_PATH_MAX];
    struct stat info;
    if (!suffix_batch_id(entry->d_name, ".ndjson.acked", batch_id) ||
        !make_path(impl, batch_id, ".ndjson.acked", data_path) ||
        !make_path(impl, batch_id, ".manifest", manifest_path) ||
        stat(data_path, &info) != 0 || now < info.st_mtime ||
        (uint64_t)(now - info.st_mtime) < LT_BATCH_STORE_ACK_RETENTION_S) continue;
    if (unlink(data_path) == 0) {
      (void)unlink(manifest_path);
      impl->health.acked_deleted++;
    }
  }
  (void)closedir(directory);
}

void lt_batch_store_maintain(lt_batch_store_t *store) {
  store_impl_t *impl;
  DIR *directory;
  struct dirent *entry;
  char oldest_id[37];
  time_t oldest_time = 0;
  struct stat info;
  char path[LT_BATCH_STORE_FILE_PATH_MAX];
  char manifest_path[LT_BATCH_STORE_FILE_PATH_MAX];
  if (store == NULL) return;
  impl = store_impl(store);
  delete_expired_acked(impl);
  (void)refresh_free_space(impl);
  while (impl->health.low_space) {
    directory = opendir(impl->root_path);
    if (directory == NULL) return;
    oldest_id[0] = '\0';
    while ((entry = readdir(directory)) != NULL) {
      char batch_id[37];
      if (!suffix_batch_id(entry->d_name, ".ndjson.acked", batch_id) ||
          !make_path(impl, batch_id, ".ndjson.acked", path) || stat(path, &info) != 0)
        continue;
      if (oldest_id[0] == '\0' || info.st_mtime < oldest_time) {
        memcpy(oldest_id, batch_id, sizeof(oldest_id));
        oldest_time = info.st_mtime;
      }
    }
    (void)closedir(directory);
    if (oldest_id[0] == '\0' || !make_path(impl, oldest_id, ".ndjson.acked", path) ||
        !make_path(impl, oldest_id, ".manifest", manifest_path) || unlink(path) != 0)
      return;
    (void)unlink(manifest_path);
    impl->health.acked_deleted++;
    if (!refresh_free_space(impl)) return;
  }
}

lt_batch_store_health_t lt_batch_store_health(const lt_batch_store_t *store) {
  return store == NULL ? (lt_batch_store_health_t){0} : const_store_impl(store)->health;
}
