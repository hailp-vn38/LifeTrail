#pragma once

#include <stdbool.h>

#include "lifetrail_batch_store.h"

bool lt_sync_ack_matches(const lt_batch_store_ready_t *batch, const char *body);
