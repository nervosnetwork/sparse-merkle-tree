//
// C implementation of SMT verification:
// https://github.com/nervosnetwork/sparse-merkle-tree
//
// origin from:
// https://github.com/nervosnetwork/godwoken/blob/6c9b92b9b06068a8678864b35a3272545ed7909e/c/gw_smt.h#L1
#ifndef _CKB_SPARSE_MERKLE_TREE_H_
#define _CKB_SPARSE_MERKLE_TREE_H_

// The faster version of memset & memcpy implementations used here are from
// the awesome musl libc project: https://www.musl-libc.org/
void *_smt_fast_memset(void *dest, int c, size_t n)
{
	unsigned char *s = (unsigned char *)dest;
	size_t k;

	/* Fill head and tail with minimal branching. Each
	 * conditional ensures that all the subsequently used
	 * offsets are well-defined and in the dest region. */

	if (!n) return dest;
	s[0] = c;
	s[n-1] = c;
	if (n <= 2) return dest;
	s[1] = c;
	s[2] = c;
	s[n-2] = c;
	s[n-3] = c;
	if (n <= 6) return dest;
	s[3] = c;
	s[n-4] = c;
	if (n <= 8) return dest;

	/* Advance pointer to align it at a 4-byte boundary,
	 * and truncate n to a multiple of 4. The previous code
	 * already took care of any head/tail that get cut off
	 * by the alignment. */

	k = -(uintptr_t)s & 3;
	s += k;
	n -= k;
	n &= -4;

#ifdef __GNUC__
	typedef uint32_t __attribute__((__may_alias__)) u32;
	typedef uint64_t __attribute__((__may_alias__)) u64;

	u32 c32 = ((u32)-1)/255 * (unsigned char)c;

	/* In preparation to copy 32 bytes at a time, aligned on
	 * an 8-byte bounary, fill head/tail up to 28 bytes each.
	 * As in the initial byte-based head/tail fill, each
	 * conditional below ensures that the subsequent offsets
	 * are valid (e.g. !(n<=24) implies n>=28). */

	*(u32 *)(s+0) = c32;
	*(u32 *)(s+n-4) = c32;
	if (n <= 8) return dest;
	*(u32 *)(s+4) = c32;
	*(u32 *)(s+8) = c32;
	*(u32 *)(s+n-12) = c32;
	*(u32 *)(s+n-8) = c32;
	if (n <= 24) return dest;
	*(u32 *)(s+12) = c32;
	*(u32 *)(s+16) = c32;
	*(u32 *)(s+20) = c32;
	*(u32 *)(s+24) = c32;
	*(u32 *)(s+n-28) = c32;
	*(u32 *)(s+n-24) = c32;
	*(u32 *)(s+n-20) = c32;
	*(u32 *)(s+n-16) = c32;

	/* Align to a multiple of 8 so we can fill 64 bits at a time,
	 * and avoid writing the same bytes twice as much as is
	 * practical without introducing additional branching. */

	k = 24 + ((uintptr_t)s & 4);
	s += k;
	n -= k;

	/* If this loop is reached, 28 tail bytes have already been
	 * filled, so any remainder when n drops below 32 can be
	 * safely ignored. */

	u64 c64 = c32 | ((u64)c32 << 32);
	for (; n >= 32; n-=32, s+=32) {
		*(u64 *)(s+0) = c64;
		*(u64 *)(s+8) = c64;
		*(u64 *)(s+16) = c64;
		*(u64 *)(s+24) = c64;
	}
#else
	/* Pure C fallback with no aliasing violations. */
	for (; n; n--, s++) *s = c;
#endif

	return dest;
}

void *_smt_fast_memcpy(void *restrict dest, const void *restrict src, size_t n)
{
	unsigned char *d = (unsigned char *)dest;
	const unsigned char *s = (unsigned char *)src;

#ifdef __GNUC__

#if __BYTE_ORDER == __LITTLE_ENDIAN
#define LS >>
#define RS <<
#else
#define LS <<
#define RS >>
#endif

	typedef uint32_t __attribute__((__may_alias__)) u32;
	uint32_t w, x;

	for (; (uintptr_t)s % 4 && n; n--) *d++ = *s++;

	if ((uintptr_t)d % 4 == 0) {
		for (; n>=16; s+=16, d+=16, n-=16) {
			*(u32 *)(d+0) = *(u32 *)(s+0);
			*(u32 *)(d+4) = *(u32 *)(s+4);
			*(u32 *)(d+8) = *(u32 *)(s+8);
			*(u32 *)(d+12) = *(u32 *)(s+12);
		}
		if (n&8) {
			*(u32 *)(d+0) = *(u32 *)(s+0);
			*(u32 *)(d+4) = *(u32 *)(s+4);
			d += 8; s += 8;
		}
		if (n&4) {
			*(u32 *)(d+0) = *(u32 *)(s+0);
			d += 4; s += 4;
		}
		if (n&2) {
			*d++ = *s++; *d++ = *s++;
		}
		if (n&1) {
			*d = *s;
		}
		return dest;
	}

	if (n >= 32) switch ((uintptr_t)d % 4) {
	case 1:
		w = *(u32 *)s;
		*d++ = *s++;
		*d++ = *s++;
		*d++ = *s++;
		n -= 3;
		for (; n>=17; s+=16, d+=16, n-=16) {
			x = *(u32 *)(s+1);
			*(u32 *)(d+0) = (w LS 24) | (x RS 8);
			w = *(u32 *)(s+5);
			*(u32 *)(d+4) = (x LS 24) | (w RS 8);
			x = *(u32 *)(s+9);
			*(u32 *)(d+8) = (w LS 24) | (x RS 8);
			w = *(u32 *)(s+13);
			*(u32 *)(d+12) = (x LS 24) | (w RS 8);
		}
		break;
	case 2:
		w = *(u32 *)s;
		*d++ = *s++;
		*d++ = *s++;
		n -= 2;
		for (; n>=18; s+=16, d+=16, n-=16) {
			x = *(u32 *)(s+2);
			*(u32 *)(d+0) = (w LS 16) | (x RS 16);
			w = *(u32 *)(s+6);
			*(u32 *)(d+4) = (x LS 16) | (w RS 16);
			x = *(u32 *)(s+10);
			*(u32 *)(d+8) = (w LS 16) | (x RS 16);
			w = *(u32 *)(s+14);
			*(u32 *)(d+12) = (x LS 16) | (w RS 16);
		}
		break;
	case 3:
		w = *(u32 *)s;
		*d++ = *s++;
		n -= 1;
		for (; n>=19; s+=16, d+=16, n-=16) {
			x = *(u32 *)(s+3);
			*(u32 *)(d+0) = (w LS 8) | (x RS 24);
			w = *(u32 *)(s+7);
			*(u32 *)(d+4) = (x LS 8) | (w RS 24);
			x = *(u32 *)(s+11);
			*(u32 *)(d+8) = (w LS 8) | (x RS 24);
			w = *(u32 *)(s+15);
			*(u32 *)(d+12) = (x LS 8) | (w RS 24);
		}
		break;
	}
	if (n&16) {
		*d++ = *s++; *d++ = *s++; *d++ = *s++; *d++ = *s++;
		*d++ = *s++; *d++ = *s++; *d++ = *s++; *d++ = *s++;
		*d++ = *s++; *d++ = *s++; *d++ = *s++; *d++ = *s++;
		*d++ = *s++; *d++ = *s++; *d++ = *s++; *d++ = *s++;
	}
	if (n&8) {
		*d++ = *s++; *d++ = *s++; *d++ = *s++; *d++ = *s++;
		*d++ = *s++; *d++ = *s++; *d++ = *s++; *d++ = *s++;
	}
	if (n&4) {
		*d++ = *s++; *d++ = *s++; *d++ = *s++; *d++ = *s++;
	}
	if (n&2) {
		*d++ = *s++; *d++ = *s++;
	}
	if (n&1) {
		*d = *s;
	}
	return dest;
#endif

	for (; n; n--) *d++ = *s++;
	return dest;
}

// users can define a new stack size if needed
#ifndef SMT_STACK_SIZE
#define SMT_STACK_SIZE 257
#endif

#define SMT_KEY_BYTES 32
#define SMT_VALUE_BYTES 32

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
    _smt_fast_memcpy(state->pairs[state->len].key, key, SMT_KEY_BYTES);
    _smt_fast_memcpy(state->pairs[state->len].value, value, SMT_KEY_BYTES);
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

  _smt_fast_memcpy(state->pairs[i].value, value, SMT_VALUE_BYTES);
  return 0;
}

int smt_state_fetch(smt_state_t *state, const uint8_t *key, uint8_t *value) {
  int32_t i = state->len - 1;
  for (; i >= 0; i--) {
    if (memcmp(key, state->pairs[i].key, SMT_KEY_BYTES) == 0) {
      _smt_fast_memcpy(value, state->pairs[i].value, SMT_VALUE_BYTES);
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
      _smt_fast_memcpy(state->pairs[sorted].key, state->pairs[item_index].key,
             SMT_KEY_BYTES);
      _smt_fast_memcpy(state->pairs[sorted].value, state->pairs[item_index].value,
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
  _smt_fast_memset(source, 0, first_byte);
  for (int i = first_byte * 8; i < first_kept_bit; i++) {
    _smt_clear_bit(source, i);
  }
}

void _smt_parent_path(uint8_t *key, uint8_t height) {
  if (height == 255) {
    _smt_fast_memset(key, 0, 32);
  } else {
    _smt_copy_bits(key, height + 1);
  }
}

int _smt_is_zero_hash(const uint8_t *value) {
  for (int i = 0; i < 32; i++) {
    if (value[i] != 0) {
      return 0;
    }
  }
  return 1;
}

#define _SMT_MERGE_VALUE_ZERO 0
#define _SMT_MERGE_VALUE_VALUE 1
#define _SMT_MERGE_VALUE_MERGE_WITH_ZERO 2

typedef struct {
  /*
   * A _smt_merge_value_t typed value might be in any of the following
   * 3 types:
   *
   * * When t is _SMT_MERGE_VALUE_ZERO, current variable represents a zero
   * hash. value will still be set to all zero, it's just that testing t
   * provides a quicker way to check against zero hashes
   * * When t is _SMT_MERGE_VALUE_VALUE, current variable represents a non-zero
   * hash value.
   * * When t is _SMT_MERGE_VALUE_MERGE_WITH_ZERO, current variable represents
   * a hash which is combined from a base node value with multiple zeros.
   */
  uint8_t t;

  uint8_t value[SMT_VALUE_BYTES];
  uint8_t zero_bits[SMT_KEY_BYTES];
  uint8_t zero_count;
} _smt_merge_value_t;

void _smt_merge_value_zero(_smt_merge_value_t *out) {
  out->t = _SMT_MERGE_VALUE_ZERO;
  _smt_fast_memset(out->value, 0, SMT_VALUE_BYTES);
}

void _smt_merge_value_from_h256(const uint8_t *v, _smt_merge_value_t *out) {
  if (_smt_is_zero_hash(v)) {
    _smt_merge_value_zero(out);
  } else {
    out->t = _SMT_MERGE_VALUE_VALUE;
    _smt_fast_memcpy(out->value, v, SMT_VALUE_BYTES);
  }
}

int _smt_merge_value_is_zero(const _smt_merge_value_t *v) {
  return v->t == _SMT_MERGE_VALUE_ZERO;
}

const uint8_t _SMT_MERGE_NORMAL = 1;
const uint8_t _SMT_MERGE_ZEROS = 2;

/* Hash base node into a H256 */
void _smt_hash_base_node(uint8_t base_height, const uint8_t *base_key,
                         const uint8_t *base_value,
                         uint8_t out[SMT_VALUE_BYTES]) {
  blake2b_state blake2b_ctx;
  blake2b_init(&blake2b_ctx, SMT_VALUE_BYTES);

  blake2b_update(&blake2b_ctx, &base_height, 1);
  blake2b_update(&blake2b_ctx, base_key, SMT_KEY_BYTES);
  blake2b_update(&blake2b_ctx, base_value, SMT_VALUE_BYTES);
  blake2b_final(&blake2b_ctx, out, SMT_VALUE_BYTES);
}

void _smt_merge_value_hash(const _smt_merge_value_t *v, uint8_t *out) {
  if (v->t == _SMT_MERGE_VALUE_MERGE_WITH_ZERO) {
    blake2b_state blake2b_ctx;
    blake2b_init(&blake2b_ctx, SMT_VALUE_BYTES);

    blake2b_update(&blake2b_ctx, &_SMT_MERGE_ZEROS, 1);
    blake2b_update(&blake2b_ctx, v->value, SMT_VALUE_BYTES);
    blake2b_update(&blake2b_ctx, v->zero_bits, SMT_KEY_BYTES);
    blake2b_update(&blake2b_ctx, &(v->zero_count), 1);
    blake2b_final(&blake2b_ctx, out, SMT_VALUE_BYTES);
  } else {
    _smt_fast_memcpy(out, v->value, SMT_VALUE_BYTES);
  }
}

void _smt_merge_with_zero(uint8_t height, const uint8_t *node_key,
                          const _smt_merge_value_t *v, int set_bit,
                          _smt_merge_value_t *out) {
  if (v->t == _SMT_MERGE_VALUE_MERGE_WITH_ZERO) {
    if (out != v) {
      _smt_fast_memcpy(out, v, sizeof(_smt_merge_value_t));
    }
    if (set_bit) {
      _smt_set_bit(out->zero_bits, height);
    }
    out->zero_count++;
  } else {
    out->t = _SMT_MERGE_VALUE_MERGE_WITH_ZERO;
    _smt_hash_base_node(height, node_key, v->value, out->value);
    _smt_fast_memset(out->zero_bits, 0, 32);
    if (set_bit) {
      _smt_set_bit(out->zero_bits, height);
    }
    out->zero_count = 1;
  }
}

/* Notice that output might collide with one of lhs, or rhs */
void _smt_merge(uint8_t height, const uint8_t *node_key,
                const _smt_merge_value_t *lhs,
                const _smt_merge_value_t *rhs,
                _smt_merge_value_t *out) {
  int lhs_zero = _smt_merge_value_is_zero(lhs);
  int rhs_zero = _smt_merge_value_is_zero(rhs);

  if (lhs_zero && rhs_zero) {
    _smt_merge_value_zero(out);
    return;
  }
  if (lhs_zero) {
    _smt_merge_with_zero(height, node_key, rhs, 1, out);
    return;
  }
  if (rhs_zero) {
    _smt_merge_with_zero(height, node_key, lhs, 0, out);
    return;
  }

  blake2b_state blake2b_ctx;
  blake2b_init(&blake2b_ctx, SMT_VALUE_BYTES);
  uint8_t data[SMT_VALUE_BYTES];

  blake2b_update(&blake2b_ctx, &_SMT_MERGE_NORMAL, 1);
  blake2b_update(&blake2b_ctx, &height, 1);
  blake2b_update(&blake2b_ctx, node_key, SMT_KEY_BYTES);
  _smt_merge_value_hash(lhs, data);
  blake2b_update(&blake2b_ctx, data, SMT_VALUE_BYTES);
  _smt_merge_value_hash(rhs, data);
  blake2b_update(&blake2b_ctx, data, SMT_VALUE_BYTES);

  blake2b_final(&blake2b_ctx, data, SMT_VALUE_BYTES);
  _smt_merge_value_from_h256(data, out);
}

const _smt_merge_value_t SMT_ZERO = {
  .t = _SMT_MERGE_VALUE_ZERO,
  .value = {0}
};

/*
 * Theoretically, a stack size of x should be able to process as many as
 * 2 ** (x - 1) updates. In this case with a stack size of 32, we can deal
 * with 2 ** 31 == 2147483648 updates, which is more than enough.
 */
int smt_calculate_root(uint8_t *buffer, const smt_state_t *pairs,
                       const uint8_t *proof, uint32_t proof_length) {
  uint8_t stack_keys[SMT_STACK_SIZE][SMT_KEY_BYTES];
  _smt_merge_value_t stack_values[SMT_STACK_SIZE];
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
        _smt_fast_memcpy(stack_keys[stack_top], pairs->pairs[leave_index].key,
               SMT_KEY_BYTES);
        _smt_merge_value_from_h256(pairs->pairs[leave_index].value, &stack_values[stack_top]);
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
        _smt_merge_value_t sibling_node;
        _smt_merge_value_from_h256(&proof[proof_index], &sibling_node);
        proof_index += 32;
        uint8_t *key = stack_keys[stack_top - 1];
        _smt_merge_value_t *value = &stack_values[stack_top - 1];
        uint16_t height = stack_heights[stack_top - 1];
        uint16_t *height_ptr = &stack_heights[stack_top - 1];
        if (height > 255) {
          return ERROR_INVALID_PROOF;
        }
        uint8_t parent_key[SMT_KEY_BYTES];
        _smt_fast_memcpy(parent_key, key, SMT_KEY_BYTES);
        _smt_parent_path(parent_key, height);

        // push value
        if (_smt_get_bit(key, height)) {
          _smt_merge((uint8_t)height, parent_key, &sibling_node, value, value);
        } else {
          _smt_merge((uint8_t)height, parent_key, value, &sibling_node, value);
        }
        // push key
        _smt_parent_path(key, height);
        // push height
        *height_ptr = height + 1;
      } break;
      case 0x51: {
        if (stack_top == 0) {
          return ERROR_INVALID_STACK;
        }
        if (proof_index + 65 > proof_length) {
          return ERROR_INVALID_PROOF;
        }
        _smt_merge_value_t sibling_node;
        sibling_node.t = _SMT_MERGE_VALUE_MERGE_WITH_ZERO;
        sibling_node.zero_count = proof[proof_index];
        _smt_fast_memcpy(&sibling_node.value, &proof[proof_index + 1], 32);
        _smt_fast_memcpy(&sibling_node.zero_bits, &proof[proof_index + 33], 32);
        proof_index += 65;
        uint8_t *key = stack_keys[stack_top - 1];
        _smt_merge_value_t *value = &stack_values[stack_top - 1];
        uint16_t height = stack_heights[stack_top - 1];
        uint16_t *height_ptr = &stack_heights[stack_top - 1];
        if (height > 255) {
          return ERROR_INVALID_PROOF;
        }
        uint8_t parent_key[SMT_KEY_BYTES];
        _smt_fast_memcpy(parent_key, key, SMT_KEY_BYTES);
        _smt_parent_path(parent_key, height);

        // push value
        if (_smt_get_bit(key, height)) {
          _smt_merge((uint8_t)height, parent_key, &sibling_node, value, value);
        } else {
          _smt_merge((uint8_t)height, parent_key, value, &sibling_node, value);
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
        uint16_t *height_a_ptr = &stack_heights[stack_top - 2];

        uint16_t height_a = stack_heights[stack_top - 2];
        uint8_t *key_a = stack_keys[stack_top - 2];
        _smt_merge_value_t *value_a = &stack_values[stack_top - 2];

        uint16_t height_b = stack_heights[stack_top - 1];
        uint8_t *key_b = stack_keys[stack_top - 1];
        _smt_merge_value_t *value_b = &stack_values[stack_top - 1];
        stack_top -= 2;
        if (height_a != height_b) {
          return ERROR_INVALID_PROOF;
        }
        if (height_a > 255) {
          return ERROR_INVALID_PROOF;
        }
        uint8_t parent_key[SMT_KEY_BYTES];
        _smt_fast_memcpy(parent_key, key_a, SMT_KEY_BYTES);
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
        _smt_fast_memcpy(key_a, parent_key, SMT_KEY_BYTES);
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
        _smt_merge_value_t *value = &stack_values[stack_top - 1];
        if (base_height > 255) {
          return ERROR_INVALID_PROOF;
        }
        uint8_t parent_key[SMT_KEY_BYTES];
        _smt_fast_memcpy(parent_key, key, SMT_KEY_BYTES);
        uint16_t height_u16 = base_height;
        for (uint16_t idx = 0; idx < zero_count; idx++) {
          height_u16 = base_height + idx;
          if (height_u16 > 255) {
            return ERROR_INVALID_PROOF;
          }
          // the following code can be omitted:
          // _smt_fast_memcpy(parent_key, key, SMT_KEY_BYTES);
          // A key's parent's parent can be calculated from parent.
          // it's not needed to do it from scratch.
          // Make sure height_u16 is in increase order
          _smt_parent_path(parent_key, (uint8_t)height_u16);
          // push value
          if (_smt_get_bit(key, (uint8_t)height_u16)) {
            _smt_merge((uint8_t)height_u16, parent_key, &SMT_ZERO, value, value);
          } else {
            _smt_merge((uint8_t)height_u16, parent_key, value, &SMT_ZERO, value);
          }
        }
        // push key
        _smt_fast_memcpy(key, parent_key, SMT_KEY_BYTES);
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

  _smt_merge_value_hash(&stack_values[0], buffer);
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
