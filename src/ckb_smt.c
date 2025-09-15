// #define CKB_C_STDLIB_PRINTF
// #include <stdio.h>

#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#ifdef INCLUDE_BLAKE2B_IMPL
#include "blake2b.h"
#else
#include "blake2b_decl_only.h"
#endif

#include "ckb_smt.h"

smt_state_t* smt_state_new(uint32_t capacity) {
  smt_state_t *state = (smt_state_t *)malloc(sizeof(smt_state_t));
  smt_pair_t *buffer = (smt_pair_t *)malloc(sizeof(smt_pair_t) * capacity);
  smt_state_init(state, buffer, capacity);
  return state;
}

uint32_t smt_state_len(smt_state_t *state) {
  return state->len;
}
