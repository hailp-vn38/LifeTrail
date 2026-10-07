#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "lifetrail_gps_batch.h"

#define LT_BATCH_STORE_PATH_MAX 256U
#define LT_BATCH_STORE_READY_PATH_MAX (LT_BATCH_STORE_PATH_MAX + 50U)
#define LT_BATCH_STORE_LOW_SPACE_BYTES (UINT64_C(16) * 1024U * 1024U)
#define LT_BATCH_STORE_PAUSE_BYTES (UINT64_C(4) * 1024U * 1024U)
#define LT_BATCH_STORE_RESUME_BYTES (UINT64_C(8) * 1024U * 1024U)
#define LT_BATCH_STORE_ACK_RETENTION_S UINT64_C(86400)

typedef bool (*lt_batch_store_free_space_t)(void *context,
                                            uint64_t *free_bytes);

typedef struct {
  const char *root_path;
  lt_batch_store_free_space_t free_space;
  void *free_space_context;
  uint64_t flush_interval_ms;
  uint64_t fsync_interval_ms;
} lt_batch_store_config_t;

typedef struct {
  uint32_t recovered_open;
  uint32_t rebuilt_manifest;
  uint32_t quarantined;
  uint32_t acked_deleted;
  uint64_t free_bytes;
  bool low_space;
  bool recording_paused;
  uint64_t sd_append_count;
  uint64_t sd_flush_count;
  uint64_t sd_fsync_count;
} lt_batch_store_health_t;

typedef struct {
  char batch_id[37];
  uint64_t byte_length;
  uint32_t record_count;
  char sha256[65];
} lt_batch_store_ready_t;

typedef struct {
  union {
    uint8_t bytes[640];
    double align_double;
    void *align_pointer;
  } state;
} lt_batch_store_t;

bool lt_batch_store_init(lt_batch_store_t *store,
                         const lt_batch_store_config_t *config);
lt_gps_batch_sink_t lt_batch_store_writer_sink(lt_batch_store_t *store);
bool lt_batch_store_mark_acked(lt_batch_store_t *store,
                               const char batch_id[37]);
bool lt_batch_store_quarantine(lt_batch_store_t *store,
                               const char batch_id[37]);
bool lt_batch_store_next_ready(lt_batch_store_t *store,
                               lt_batch_store_ready_t *ready);
bool lt_batch_store_ready_path(const lt_batch_store_t *store,
                               const lt_batch_store_ready_t *ready,
                               char path[LT_BATCH_STORE_READY_PATH_MAX]);
void lt_batch_store_maintain(lt_batch_store_t *store);
lt_batch_store_health_t lt_batch_store_health(const lt_batch_store_t *store);
/* Call on the storage task, even when there are no accepted GPS Records. */
bool lt_batch_store_poll(lt_batch_store_t *store, uint64_t monotonic_ms, bool force_sync);
bool lt_batch_store_close(lt_batch_store_t *store);
