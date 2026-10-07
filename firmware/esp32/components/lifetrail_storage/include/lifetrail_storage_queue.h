#pragma once

#include "lifetrail_gps_policy.h"

#define LT_STORAGE_QUEUE_CAPACITY 256U

typedef struct {
  lt_gps_record_t record;
  lt_gps_motion_state_t motion;
} lt_storage_entry_t;

/* Callbacks guard RAM operations only; never hold the lock during SD I/O. */
typedef void (*lt_storage_queue_lock_t)(void *context);
typedef struct {
  lt_storage_entry_t *entries;
  size_t capacity;
  size_t start;
  size_t count;
  uint64_t overflow_count;
  lt_storage_queue_lock_t lock;
  lt_storage_queue_lock_t unlock;
  void *lock_context;
} lt_storage_queue_t;

void lt_storage_queue_init(lt_storage_queue_t *queue, lt_storage_entry_t *entries,
                            size_t capacity, lt_storage_queue_lock_t lock,
                            lt_storage_queue_lock_t unlock, void *lock_context);
void lt_storage_queue_push(lt_storage_queue_t *queue, const lt_storage_entry_t *entry);
bool lt_storage_queue_pop(lt_storage_queue_t *queue, lt_storage_entry_t *entry);
uint64_t lt_storage_queue_overflow_count(lt_storage_queue_t *queue);
