#pragma once

#include <stddef.h>
#include <stdint.h>

typedef struct {
  uint32_t state[8];
  uint64_t bit_length;
  uint8_t block[64];
  size_t block_length;
} lt_sha256_t;

void lt_sha256_init(lt_sha256_t *sha);
void lt_sha256_update(lt_sha256_t *sha, const uint8_t *data, size_t length);
void lt_sha256_final(lt_sha256_t *sha, uint8_t digest[32]);
void lt_sha256_hex(const uint8_t digest[32], char hex[65]);
