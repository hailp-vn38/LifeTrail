#include "lifetrail_gps_batch.h"

#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

typedef struct {
  lt_gps_batch_settings_t settings;
  lt_gps_batch_id_source_t id_source;
  void *id_source_context;
  lt_gps_batch_sink_t sink;
  int64_t first_ts_ms;
  int64_t last_ts_ms;
  size_t byte_length;
  bool active;
} writer_impl_t;

_Static_assert(sizeof(writer_impl_t) <= sizeof(lt_gps_batch_writer_t),
               "batch writer state is too small");

static writer_impl_t *writer_impl(lt_gps_batch_writer_t *writer) {
  return (writer_impl_t *)&writer->state;
}

static bool valid_record(const lt_gps_record_t *record) {
  return record != NULL && record->ts_ms > 0 && isfinite(record->lat) &&
         isfinite(record->lon) && record->lat >= -90.0 && record->lat <= 90.0 &&
         record->lon >= -180.0 && record->lon <= 180.0 &&
         (!record->has_alt_m || isfinite(record->alt_m)) &&
         (!record->has_speed_mps ||
          (isfinite(record->speed_mps) && record->speed_mps >= 0.0)) &&
         (!record->has_course_deg ||
          (isfinite(record->course_deg) && record->course_deg >= 0.0 &&
           record->course_deg < 360.0)) &&
         (!record->has_hdop || (isfinite(record->hdop) && record->hdop >= 0.0));
}

static size_t format_optional(char *destination, size_t capacity, bool present,
                              double value) {
  int written = present ? snprintf(destination, capacity, "%.6f", value)
                        : snprintf(destination, capacity, "null");
  return written < 0 || (size_t)written >= capacity ? 0U : (size_t)written;
}

static size_t serialize_record(const lt_gps_record_t *record, char line[512]) {
  char alt_m[32];
  char speed_mps[32];
  char course_deg[32];
  char hdop[32];
  int written;

  if (format_optional(alt_m, sizeof(alt_m), record->has_alt_m, record->alt_m) ==
          0U ||
      format_optional(speed_mps, sizeof(speed_mps), record->has_speed_mps,
                      record->speed_mps) == 0U ||
      format_optional(course_deg, sizeof(course_deg), record->has_course_deg,
                      record->course_deg) == 0U ||
      format_optional(hdop, sizeof(hdop), record->has_hdop, record->hdop) ==
          0U) {
    return 0U;
  }
  written = snprintf(
      line, 512U,
      "{\"ts_ms\":%" PRId64
      ",\"lat\":%.7f,\"lon\":%.7f,\"alt_m\":%s,\"speed_mps\":%s,\"course_deg\":"
      "%s,\"fix_quality\":%u,\"satellites\":%u,\"hdop\":%s}\n",
      record->ts_ms, record->lat, record->lon, alt_m, speed_mps, course_deg,
      (unsigned int)record->fix_quality, (unsigned int)record->satellites,
      hdop);
  return written < 0 || written >= 512 ? 0U : (size_t)written;
}

static void format_uuid_v4(const uint8_t bytes[16], char destination[37]) {
  static const char hex[] = "0123456789abcdef";
  static const uint8_t groups[] = {4U, 2U, 2U, 2U, 6U};
  size_t byte_index = 0U;
  size_t group;
  size_t group_byte;
  size_t output_index = 0U;

  for (group = 0U; group < sizeof(groups); group++) {
    if (group > 0U) {
      destination[output_index++] = '-';
    }
    for (group_byte = 0U; group_byte < groups[group]; group_byte++) {
      destination[output_index++] = hex[bytes[byte_index] >> 4U];
      destination[output_index++] = hex[bytes[byte_index] & 0x0fU];
      byte_index++;
    }
  }
  destination[output_index] = '\0';
}

static bool open_batch(writer_impl_t *impl, int64_t first_ts_ms) {
  uint8_t random_bytes[16];
  char batch_id[37];

  if (impl->id_source == NULL || impl->sink.open == NULL ||
      !impl->id_source(impl->id_source_context, random_bytes)) {
    return false;
  }
  random_bytes[6] = (uint8_t)((random_bytes[6] & 0x0fU) | 0x40U);
  random_bytes[8] = (uint8_t)((random_bytes[8] & 0x3fU) | 0x80U);
  format_uuid_v4(random_bytes, batch_id);
  if (!impl->sink.open(impl->sink.context, batch_id)) {
    return false;
  }
  impl->active = true;
  impl->first_ts_ms = first_ts_ms;
  impl->byte_length = 0U;
  impl->last_ts_ms = 0;
  return true;
}

void lt_gps_batch_writer_init(lt_gps_batch_writer_t *writer,
                              const lt_gps_batch_settings_t *settings,
                              lt_gps_batch_id_source_t id_source,
                              void *id_source_context,
                              lt_gps_batch_sink_t sink) {
  writer_impl_t *impl = writer_impl(writer);
  memset(impl, 0, sizeof(*impl));
  impl->settings = settings == NULL
        ? (lt_gps_batch_settings_t){
            .max_age_moving_ms = UINT64_C(60000),
            .max_age_stationary_ms = LT_GPS_BATCH_DEFAULT_MAX_AGE_MS,
            .max_bytes = LT_GPS_BATCH_DEFAULT_MAX_BYTES,
        }
        : *settings;
  impl->id_source = id_source;
  impl->id_source_context = id_source_context;
  impl->sink = sink;
}

bool lt_gps_batch_writer_append(lt_gps_batch_writer_t *writer,
                                const lt_gps_record_t *record) {
  return lt_gps_batch_writer_append_with_motion(
      writer, record, LT_GPS_MOTION_MOVING);
}

static uint64_t max_age_for_motion(const lt_gps_batch_settings_t *settings,
                                   lt_gps_motion_state_t motion_state) {
  if (motion_state == LT_GPS_MOTION_STATIONARY &&
      settings->max_age_stationary_ms != 0U) {
    return settings->max_age_stationary_ms;
  }
  if (settings->max_age_moving_ms != 0U) return settings->max_age_moving_ms;
  return settings->max_age_ms;
}

bool lt_gps_batch_writer_append_with_motion(
    lt_gps_batch_writer_t *writer, const lt_gps_record_t *record,
    lt_gps_motion_state_t motion_state) {
  writer_impl_t *impl = writer_impl(writer);
  char line[512];
  size_t line_length;
  bool must_rotate;
  uint64_t max_age_ms = max_age_for_motion(&impl->settings, motion_state);

  if (!valid_record(record) || max_age_ms == 0U ||
      impl->settings.max_bytes == 0U || impl->sink.append_line == NULL) {
    return false;
  }
  line_length = serialize_record(record, line);
  if (line_length == 0U || line_length > impl->settings.max_bytes) {
    return false;
  }
  if (impl->active && record->ts_ms <= impl->last_ts_ms) {
    return false;
  }
  must_rotate = impl->active &&
                (record->ts_ms - impl->first_ts_ms >=
                     (int64_t)max_age_ms ||
                 line_length > impl->settings.max_bytes - impl->byte_length);
  if (must_rotate) {
    if (impl->sink.rotate == NULL || !impl->sink.rotate(impl->sink.context)) {
      return false;
    }
    impl->active = false;
  }
  if (!impl->active && !open_batch(impl, record->ts_ms)) {
    return false;
  }
  if (!impl->sink.append_line(impl->sink.context, line, line_length)) {
    return false;
  }
  impl->byte_length += line_length;
  impl->last_ts_ms = record->ts_ms;
  return true;
}
