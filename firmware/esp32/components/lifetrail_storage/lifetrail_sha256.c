#include "lifetrail_sha256.h"

#include <string.h>

#define ROTR(value, count) (((value) >> (count)) | ((value) << (32U - (count))))

static const uint32_t round_constants[64] = {
    0x428a2f98U, 0x71374491U, 0xb5c0fbcfU, 0xe9b5dba5U, 0x3956c25bU,
    0x59f111f1U, 0x923f82a4U, 0xab1c5ed5U, 0xd807aa98U, 0x12835b01U,
    0x243185beU, 0x550c7dc3U, 0x72be5d74U, 0x80deb1feU, 0x9bdc06a7U,
    0xc19bf174U, 0xe49b69c1U, 0xefbe4786U, 0x0fc19dc6U, 0x240ca1ccU,
    0x2de92c6fU, 0x4a7484aaU, 0x5cb0a9dcU, 0x76f988daU, 0x983e5152U,
    0xa831c66dU, 0xb00327c8U, 0xbf597fc7U, 0xc6e00bf3U, 0xd5a79147U,
    0x06ca6351U, 0x14292967U, 0x27b70a85U, 0x2e1b2138U, 0x4d2c6dfcU,
    0x53380d13U, 0x650a7354U, 0x766a0abbU, 0x81c2c92eU, 0x92722c85U,
    0xa2bfe8a1U, 0xa81a664bU, 0xc24b8b70U, 0xc76c51a3U, 0xd192e819U,
    0xd6990624U, 0xf40e3585U, 0x106aa070U, 0x19a4c116U, 0x1e376c08U,
    0x2748774cU, 0x34b0bcb5U, 0x391c0cb3U, 0x4ed8aa4aU, 0x5b9cca4fU,
    0x682e6ff3U, 0x748f82eeU, 0x78a5636fU, 0x84c87814U, 0x8cc70208U,
    0x90befffaU, 0xa4506cebU, 0xbef9a3f7U, 0xc67178f2U};

static void transform(lt_sha256_t *sha, const uint8_t block[64]) {
  uint32_t words[64];
  uint32_t a, b, c, d, e, f, g, h;
  size_t index;

  for (index = 0U; index < 16U; index++) {
    words[index] = ((uint32_t)block[index * 4U] << 24U) |
                   ((uint32_t)block[index * 4U + 1U] << 16U) |
                   ((uint32_t)block[index * 4U + 2U] << 8U) |
                   block[index * 4U + 3U];
  }
  for (; index < 64U; index++) {
    uint32_t s0 = ROTR(words[index - 15U], 7U) ^ ROTR(words[index - 15U], 18U) ^
                  (words[index - 15U] >> 3U);
    uint32_t s1 = ROTR(words[index - 2U], 17U) ^ ROTR(words[index - 2U], 19U) ^
                  (words[index - 2U] >> 10U);
    words[index] = words[index - 16U] + s0 + words[index - 7U] + s1;
  }
  a = sha->state[0]; b = sha->state[1]; c = sha->state[2]; d = sha->state[3];
  e = sha->state[4]; f = sha->state[5]; g = sha->state[6]; h = sha->state[7];
  for (index = 0U; index < 64U; index++) {
    uint32_t s1 = ROTR(e, 6U) ^ ROTR(e, 11U) ^ ROTR(e, 25U);
    uint32_t choice = (e & f) ^ ((~e) & g);
    uint32_t temp1 = h + s1 + choice + round_constants[index] + words[index];
    uint32_t s0 = ROTR(a, 2U) ^ ROTR(a, 13U) ^ ROTR(a, 22U);
    uint32_t majority = (a & b) ^ (a & c) ^ (b & c);
    uint32_t temp2 = s0 + majority;
    h = g; g = f; f = e; e = d + temp1; d = c; c = b; b = a; a = temp1 + temp2;
  }
  sha->state[0] += a; sha->state[1] += b; sha->state[2] += c; sha->state[3] += d;
  sha->state[4] += e; sha->state[5] += f; sha->state[6] += g; sha->state[7] += h;
}

void lt_sha256_init(lt_sha256_t *sha) {
  static const uint32_t initial[] = {0x6a09e667U, 0xbb67ae85U, 0x3c6ef372U,
                                     0xa54ff53aU, 0x510e527fU, 0x9b05688cU,
                                     0x1f83d9abU, 0x5be0cd19U};
  memset(sha, 0, sizeof(*sha));
  memcpy(sha->state, initial, sizeof(initial));
}

void lt_sha256_update(lt_sha256_t *sha, const uint8_t *data, size_t length) {
  size_t index;
  for (index = 0U; index < length; index++) {
    sha->block[sha->block_length++] = data[index];
    if (sha->block_length == sizeof(sha->block)) {
      transform(sha, sha->block);
      sha->bit_length += 512U;
      sha->block_length = 0U;
    }
  }
}

void lt_sha256_final(lt_sha256_t *sha, uint8_t digest[32]) {
  size_t index = sha->block_length;
  uint64_t bit_length;
  sha->block[index++] = 0x80U;
  if (index > 56U) {
    while (index < 64U) sha->block[index++] = 0U;
    transform(sha, sha->block);
    index = 0U;
  }
  while (index < 56U) sha->block[index++] = 0U;
  bit_length = sha->bit_length + (uint64_t)sha->block_length * 8U;
  for (index = 0U; index < 8U; index++)
    sha->block[63U - index] = (uint8_t)(bit_length >> (index * 8U));
  transform(sha, sha->block);
  for (index = 0U; index < 8U; index++) {
    digest[index * 4U] = (uint8_t)(sha->state[index] >> 24U);
    digest[index * 4U + 1U] = (uint8_t)(sha->state[index] >> 16U);
    digest[index * 4U + 2U] = (uint8_t)(sha->state[index] >> 8U);
    digest[index * 4U + 3U] = (uint8_t)sha->state[index];
  }
}

void lt_sha256_hex(const uint8_t digest[32], char hex[65]) {
  static const char alphabet[] = "0123456789abcdef";
  size_t index;
  for (index = 0U; index < 32U; index++) {
    hex[index * 2U] = alphabet[digest[index] >> 4U];
    hex[index * 2U + 1U] = alphabet[digest[index] & 0x0fU];
  }
  hex[64] = '\0';
}
