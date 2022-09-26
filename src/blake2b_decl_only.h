#ifndef CKB_MISCELLANEOUS_SCRIPTS_SIMULATOR_BLAKE2B_DECL_ONLY_H_
#define CKB_MISCELLANEOUS_SCRIPTS_SIMULATOR_BLAKE2B_DECL_ONLY_H_
#include <stddef.h>
#include <stdint.h>

enum blake2b_constant {
    BLAKE2B_BLOCKBYTES = 128,
    BLAKE2B_OUTBYTES = 64,
    BLAKE2B_KEYBYTES = 64,
    BLAKE2B_SALTBYTES = 16,
    BLAKE2B_PERSONALBYTES = 16
};

typedef struct blake2b_state__ {
    uint64_t h[8];
    uint64_t t[2];
    uint64_t f[2];
    uint8_t buf[BLAKE2B_BLOCKBYTES];
    size_t buflen;
    size_t outlen;
    uint8_t last_node;
} blake2b_state;

typedef struct blake2b_param__ {
  uint8_t digest_length;                   /* 1 */
  uint8_t key_length;                      /* 2 */
  uint8_t fanout;                          /* 3 */
  uint8_t depth;                           /* 4 */
  uint32_t leaf_length;                    /* 8 */
  uint32_t node_offset;                    /* 12 */
  uint32_t xof_length;                     /* 16 */
  uint8_t node_depth;                      /* 17 */
  uint8_t inner_length;                    /* 18 */
  uint8_t reserved[14];                    /* 32 */
  uint8_t salt[BLAKE2B_SALTBYTES];         /* 48 */
  uint8_t personal[BLAKE2B_PERSONALBYTES]; /* 64 */
} blake2b_param;

/* Streaming API */
//int ckb_blake2b_init(blake2b_state *S, size_t outlen);
int blake2b_init(blake2b_state *S, size_t outlen);
int blake2b_init_param(blake2b_state *S, const blake2b_param *P);
int blake2b_init_key(blake2b_state *S, size_t outlen, const void *key,
                     size_t keylen);
int blake2b_update(blake2b_state *S, const void *in, size_t inlen);
int blake2b_final(blake2b_state *S, void *out, size_t outlen);
/* Simple API */
int blake2b(void *out, size_t outlen, const void *in, size_t inlen,
            const void *key, size_t keylen);

/* This is simply an alias for blake2b */
int blake2(void *out, size_t outlen, const void *in, size_t inlen,
           const void *key, size_t keylen);


static inline void store32(void *dst, uint32_t w) {
#if defined(NATIVE_LITTLE_ENDIAN)
  memcpy(dst, &w, sizeof w);
#else
  uint8_t *p = (uint8_t *)dst;
  p[0] = (uint8_t)(w >> 0);
  p[1] = (uint8_t)(w >> 8);
  p[2] = (uint8_t)(w >> 16);
  p[3] = (uint8_t)(w >> 24);
#endif
}

const char *DEFAULT_PERSONAL = "ckb-default-hash";
int ckb_blake2b_init(blake2b_state *S, size_t outlen) {
  blake2b_param P[1];

  if ((!outlen) || (outlen > BLAKE2B_OUTBYTES)) return -1;

  P->digest_length = (uint8_t)outlen;
  P->key_length = 0;
  P->fanout = 1;
  P->depth = 1;
  store32(&P->leaf_length, 0);
  store32(&P->node_offset, 0);
  store32(&P->xof_length, 0);
  P->node_depth = 0;
  P->inner_length = 0;
  memset(P->reserved, 0, sizeof(P->reserved));
  memset(P->salt, 0, sizeof(P->salt));
  memset(P->personal, 0, sizeof(P->personal));
  for (int i = 0; i < BLAKE2B_PERSONALBYTES; ++i) {
    (P->personal)[i] = DEFAULT_PERSONAL[i];
  }
  return blake2b_init_param(S, P);
}

#endif  // CKB_MISCELLANEOUS_SCRIPTS_SIMULATOR_BLAKE2B_DECL_ONLY_H_
