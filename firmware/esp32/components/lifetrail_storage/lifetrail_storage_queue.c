#include "lifetrail_storage_queue.h"

#include <string.h>

static void lock_queue(lt_storage_queue_t *queue) {
  if (queue->lock != NULL) queue->lock(queue->lock_context);
}
static void unlock_queue(lt_storage_queue_t *queue) {
  if (queue->unlock != NULL) queue->unlock(queue->lock_context);
}

void lt_storage_queue_init(lt_storage_queue_t *queue, lt_storage_entry_t *entries,
                            size_t capacity, lt_storage_queue_lock_t lock,
                            lt_storage_queue_lock_t unlock, void *lock_context) {
  memset(queue, 0, sizeof(*queue));
  queue->entries = entries;
  queue->capacity = capacity;
  queue->lock = lock;
  queue->unlock = unlock;
  queue->lock_context = lock_context;
}

void lt_storage_queue_push(lt_storage_queue_t *queue, const lt_storage_entry_t *entry) {
  lock_queue(queue);
  if (queue->capacity == 0U) {
    queue->overflow_count++;
  } else {
    if (queue->count == queue->capacity) {
      queue->start = (queue->start + 1U) % queue->capacity;
      queue->count--;
      queue->overflow_count++;
    }
    size_t index = (queue->start + queue->count) % queue->capacity;
    queue->entries[index] = *entry;
    queue->count++;
  }
  unlock_queue(queue);
}

bool lt_storage_queue_pop(lt_storage_queue_t *queue, lt_storage_entry_t *entry) {
  lock_queue(queue);
  bool present = queue->count > 0U;
  if (present) {
    *entry = queue->entries[queue->start];
    queue->start = (queue->start + 1U) % queue->capacity;
    queue->count--;
  }
  unlock_queue(queue);
  return present;
}

uint64_t lt_storage_queue_overflow_count(lt_storage_queue_t *queue) {
  lock_queue(queue);
  uint64_t count = queue->overflow_count;
  unlock_queue(queue);
  return count;
}
