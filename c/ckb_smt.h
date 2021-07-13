//
// C implementation of SMT verification:
// https://github.com/nervosnetwork/sparse-merkle-tree
//
// origin from:
// https://github.com/nervosnetwork/godwoken/blob/6c9b92b9b06068a8678864b35a3272545ed7909e/c/gw_smt.h#L1
#ifndef _CKB_SPARSE_MERKLE_TREE_H_
#define _CKB_SPARSE_MERKLE_TREE_H_

// users can define a new stack size if needed
#ifndef SMT_STACK_SIZE
#define SMT_STACK_SIZE 257
#endif

#define SMT_KEY_BYTES 32
#define SMT_VALUE_BYTES 32

const uint8_t SMT_ZERO[SMT_VALUE_BYTES] = {0};

enum SMTErrorCode {
  // SMT
  ERROR_INSUFFICIENT_CAPACITY = 80,
  ERROR_NOT_FOUND,
  ERROR_INVALID_STACK,
  ERROR_INVALID_SIBLING,
  ERROR_INVALID_PROOF
};

/* Key Value Pairs */
typedef struct {
  uint8_t key[SMT_KEY_BYTES];
  uint8_t value[SMT_VALUE_BYTES];
  uint32_t order;
} smt_pair_t;

typedef struct {
  smt_pair_t *pairs;
  uint32_t len;
  uint32_t capacity;
} smt_state_t;

void smt_state_init(smt_state_t *state, smt_pair_t *buffer, uint32_t capacity) {
  state->pairs = buffer;
  state->len = 0;
  state->capacity = capacity;
}

int smt_state_insert(smt_state_t *state, const uint8_t *key,
                     const uint8_t *value) {
  if (state->len < state->capacity) {
    /* shortcut, append at end */
    memcpy(state->pairs[state->len].key, key, SMT_KEY_BYTES);
    memcpy(state->pairs[state->len].value, value, SMT_KEY_BYTES);
    state->len++;
    return 0;
  }

  /* Find a matched key and overwritten it */
  int32_t i = state->len - 1;
  for (; i >= 0; i--) {
    if (memcmp(key, state->pairs[i].key, SMT_KEY_BYTES) == 0) {
      break;
    }
  }

  if (i < 0) {
    return ERROR_INSUFFICIENT_CAPACITY;
  }

  memcpy(state->pairs[i].value, value, SMT_VALUE_BYTES);
  return 0;
}

int smt_state_fetch(smt_state_t *state, const uint8_t *key, uint8_t *value) {
  int32_t i = state->len - 1;
  for (; i >= 0; i--) {
    if (memcmp(key, state->pairs[i].key, SMT_KEY_BYTES) == 0) {
      memcpy(value, state->pairs[i].value, SMT_VALUE_BYTES);
      return 0;
    }
  }
  return ERROR_NOT_FOUND;
}

int _smt_pair_cmp(const void *a, const void *b) {
  const smt_pair_t *pa = (const smt_pair_t *)a;
  const smt_pair_t *pb = (const smt_pair_t *)b;

  for (int i = SMT_KEY_BYTES - 1; i >= 0; i--) {
    int cmp_result = pa->key[i] - pb->key[i];
    if (cmp_result != 0) {
      return cmp_result;
    }
  }
  return pa->order - pb->order;
}

void smt_state_normalize(smt_state_t *state) {
  for (uint32_t i = 0; i < state->len; i++) {
    state->pairs[i].order = state->len - i;
  }
  qsort(state->pairs, state->len, sizeof(smt_pair_t), _smt_pair_cmp);
  /* Remove duplicate ones */
  int32_t sorted = 0, next = 0;
  while (next < (int32_t)state->len) {
    int32_t item_index = next++;
    while (next < (int32_t)state->len &&
           memcmp(state->pairs[item_index].key, state->pairs[next].key,
                  SMT_KEY_BYTES) == 0) {
      next++;
    }
    if (item_index != sorted) {
      memcpy(state->pairs[sorted].key, state->pairs[item_index].key,
             SMT_KEY_BYTES);
      memcpy(state->pairs[sorted].value, state->pairs[item_index].value,
             SMT_VALUE_BYTES);
    }
    sorted++;
  }
  state->len = sorted;
}

/* SMT */

int _smt_get_bit(const uint8_t *data, int offset) {
  int byte_pos = offset / 8;
  int bit_pos = offset % 8;
  return ((data[byte_pos] >> bit_pos) & 1) != 0;
}

void _smt_set_bit(uint8_t *data, int offset) {
  int byte_pos = offset / 8;
  int bit_pos = offset % 8;
  data[byte_pos] |= 1 << bit_pos;
}

void _smt_clear_bit(uint8_t *data, int offset) {
  int byte_pos = offset / 8;
  int bit_pos = offset % 8;
  data[byte_pos] &= (uint8_t)(~(1 << bit_pos));
}

void _smt_copy_bits(uint8_t *source, int first_kept_bit) {
  int first_byte = first_kept_bit / 8;
  for (int i = 0; i < first_byte; i++) {
    source[i] = 0;
  }
  for (int i = first_byte * 8; i < first_kept_bit; i++) {
    _smt_clear_bit(source, i);
  }
}

void _smt_parent_path(uint8_t *key, uint8_t height) {
  if (height == 255) {
    memset(key, 0, 32);
  } else {
    _smt_copy_bits(key, height + 1);
  }
}

int _smt_zero_value(const uint8_t *value) {
  for (int i = 0; i < 32; i++) {
    if (value[i] != 0) {
      return 0;
    }
  }
  return 1;
}

/* Notice that output might collide with one of lhs, or rhs */
void _smt_merge(uint8_t height, const uint8_t *node_key, const uint8_t *lhs,
                const uint8_t *rhs, uint8_t *output) {
  if (_smt_zero_value(lhs) && _smt_zero_value(rhs)) {
    memcpy(output, SMT_ZERO, SMT_VALUE_BYTES);
  } else {
    blake2b_state blake2b_ctx;
    blake2b_init(&blake2b_ctx, 32);

    blake2b_update(&blake2b_ctx, &height, 1);
    blake2b_update(&blake2b_ctx, node_key, 32);
    blake2b_update(&blake2b_ctx, lhs, 32);
    blake2b_update(&blake2b_ctx, rhs, 32);

    blake2b_final(&blake2b_ctx, output, 32);
  }
}

/*
 * Theoretically, a stack size of x should be able to process as many as
 * 2 ** (x - 1) updates. In this case with a stack size of 32, we can deal
 * with 2 ** 31 == 2147483648 updates, which is more than enough.
 */
int smt_calculate_root(uint8_t *buffer, const smt_state_t *pairs,
                       const uint8_t *proof, uint32_t proof_length) {
  uint8_t stack_keys[SMT_STACK_SIZE][SMT_KEY_BYTES];
  uint8_t stack_values[SMT_STACK_SIZE][SMT_VALUE_BYTES];
  uint16_t stack_heights[SMT_STACK_SIZE] = {0};

  uint32_t proof_index = 0;
  uint32_t leave_index = 0;
  uint32_t stack_top = 0;

  while (proof_index < proof_length) {
    switch (proof[proof_index++]) {
      case 0x4C: {
        if (stack_top >= SMT_STACK_SIZE) {
          return ERROR_INVALID_STACK;
        }
        if (leave_index >= pairs->len) {
          return ERROR_INVALID_PROOF;
        }
        memcpy(stack_keys[stack_top], pairs->pairs[leave_index].key,
               SMT_KEY_BYTES);
        memcpy(stack_values[stack_top], pairs->pairs[leave_index].value,
               SMT_VALUE_BYTES);
        stack_heights[stack_top] = 0;
        stack_top++;
        leave_index++;
      } break;
      case 0x50: {
        if (stack_top == 0) {
          return ERROR_INVALID_STACK;
        }
        if (proof_index + 32 > proof_length) {
          return ERROR_INVALID_PROOF;
        }
        const uint8_t *sibling_node = &proof[proof_index];
        proof_index += 32;
        uint8_t *key = stack_keys[stack_top - 1];
        uint8_t *value = stack_values[stack_top - 1];
        uint16_t height = stack_heights[stack_top - 1];
        uint16_t *height_ptr = &stack_heights[stack_top - 1];
        if (height > 255) {
          return ERROR_INVALID_PROOF;
        }
        uint8_t parent_key[SMT_KEY_BYTES];
        memcpy(parent_key, key, SMT_KEY_BYTES);
        _smt_parent_path(parent_key, height);

        // push value
        if (_smt_get_bit(key, height)) {
          _smt_merge((uint8_t)height, parent_key, sibling_node, value, value);
        } else {
          _smt_merge((uint8_t)height, parent_key, value, sibling_node, value);
        }
        // push key
        _smt_parent_path(key, height);
        // push height
        *height_ptr = height + 1;
      } break;
      case 0x48: {
        if (stack_top < 2) {
          return ERROR_INVALID_STACK;
        }
        if (proof_index >= proof_length) {
          return ERROR_INVALID_PROOF;
        }
        uint16_t *height_a_ptr = &stack_heights[stack_top - 2];

        uint16_t height_a = stack_heights[stack_top - 2];
        uint8_t *key_a = stack_keys[stack_top - 2];
        uint8_t *value_a = stack_values[stack_top - 2];

        uint16_t height_b = stack_heights[stack_top - 1];
        uint8_t *key_b = stack_keys[stack_top - 1];
        uint8_t *value_b = stack_values[stack_top - 1];
        stack_top -= 2;
        if (height_a != height_b) {
          return ERROR_INVALID_PROOF;
        }
        if (height_a > 255) {
          return ERROR_INVALID_PROOF;
        }
        uint8_t parent_key[SMT_KEY_BYTES];
        memcpy(parent_key, key_a, SMT_KEY_BYTES);
        _smt_parent_path(parent_key, (uint8_t)height_a);

        // 2 keys should have same parent keys
        _smt_parent_path(key_b, (uint8_t)height_b);
        if (memcmp(parent_key, key_b, SMT_KEY_BYTES) != 0) {
          return ERROR_INVALID_PROOF;
        }
        // push value
        if (_smt_get_bit(key_a, height_a)) {
          _smt_merge(height_a, parent_key, value_b, value_a, value_a);
        } else {
          _smt_merge(height_a, parent_key, value_a, value_b, value_a);
        }
        // push key
        memcpy(key_a, parent_key, SMT_KEY_BYTES);
        // push height
        *height_a_ptr = height_a + 1;
        stack_top++;
      } break;
      case 0x4F: {
        if (stack_top < 1) {
          return ERROR_INVALID_STACK;
        }
        if (proof_index >= proof_length) {
          return ERROR_INVALID_PROOF;
        }
        uint16_t n = proof[proof_index];
        proof_index++;
        uint16_t zero_count = 0;
        if (n == 0) {
          zero_count = 256;
        } else {
          zero_count = n;
        }
        uint16_t *base_height_ptr = &stack_heights[stack_top - 1];
        uint16_t base_height = stack_heights[stack_top - 1];
        uint8_t *key = stack_keys[stack_top - 1];
        uint8_t *value = stack_values[stack_top - 1];
        if (base_height > 255) {
          return ERROR_INVALID_PROOF;
        }
        uint8_t parent_key[SMT_KEY_BYTES];
        memcpy(parent_key, key, SMT_KEY_BYTES);
        uint16_t height_u16 = base_height;
        for (uint16_t idx = 0; idx < zero_count; idx++) {
          height_u16 = base_height + idx;
          if (height_u16 > 255) {
            return ERROR_INVALID_PROOF;
          }
          // the following code can be omitted:
          // memcpy(parent_key, key, SMT_KEY_BYTES);
          // A key's parent's parent can be calculated from parent.
          // it's not needed to do it from scratch.
          // Make sure height_u16 is in increase order
          _smt_parent_path(parent_key, (uint8_t)height_u16);
          // push value
          if (_smt_get_bit(key, (uint8_t)height_u16)) {
            _smt_merge((uint8_t)height_u16, parent_key, SMT_ZERO, value, value);
          } else {
            _smt_merge((uint8_t)height_u16, parent_key, value, SMT_ZERO, value);
          }
        }
        // push key
        memcpy(key, parent_key, SMT_KEY_BYTES);
        // push height
        *base_height_ptr = height_u16 + 1;
      } break;
      default:
        return ERROR_INVALID_PROOF;
    }
  }
  if (stack_top != 1) {
    return ERROR_INVALID_STACK;
  }
  if (stack_heights[0] != 256) {
    return ERROR_INVALID_PROOF;
  }
  /* All leaves must be used */
  if (leave_index != pairs->len) {
    return ERROR_INVALID_PROOF;
  }

  memcpy(buffer, stack_values[0], 32);
  return 0;
}

int smt_verify(const uint8_t *hash, const smt_state_t *state,
               const uint8_t *proof, uint32_t proof_length) {
  uint8_t buffer[32];
  int ret = smt_calculate_root(buffer, state, proof, proof_length);
  if (ret != 0) {
    return ret;
  }
  if (memcmp(buffer, hash, 32) != 0) {
    return ERROR_INVALID_PROOF;
  }
  return 0;
}

#endif
