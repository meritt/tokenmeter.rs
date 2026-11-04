#ifndef TOKENMETER_H
#define TOKENMETER_H

#include <stddef.h>
#include <stdint.h>

#define TOKENMETER_OK 0

#define TOKENMETER_INVALID_UTF8 1

#define TOKENMETER_INVALID_ARGUMENT 2

#define TOKENMETER_INTERNAL 3

#define TOKENMETER_NOT_UTF8 UINT32_MAX

typedef struct tokenmeter_groups {
  uint32_t status;
  uint32_t tokens;
  uint32_t truncated;
  uint32_t *groups;
  size_t groups_len;
} tokenmeter_groups;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

uint32_t tokenmeter_count(const uint8_t *input, size_t input_len);

void tokenmeter_split(const uint8_t *input,
                      size_t input_len,
                      size_t limit,
                      struct tokenmeter_groups *out);

void tokenmeter_split_free(struct tokenmeter_groups *split);

void tokenmeter_prepare(void);

const char *tokenmeter_encoding(void);

const char *tokenmeter_version(void);

uint8_t *tokenmeter_alloc(size_t len);

void tokenmeter_dealloc(uint8_t *p, size_t len);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* TOKENMETER_H */
