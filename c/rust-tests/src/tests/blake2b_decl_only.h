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

/* Streaming API */
int ckb_blake2b_init(blake2b_state *S, size_t outlen);
int blake2b_init(blake2b_state *S, size_t outlen);
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

#endif  // CKB_MISCELLANEOUS_SCRIPTS_SIMULATOR_BLAKE2B_DECL_ONLY_H_
