#ifndef NVMA_LIB_H
#define NVMA_LIB_H

#include <stdint.h>
#include <stddef.h>

int nvma_assemble(const char *asm_text, size_t asm_len,
                  uint8_t **out_buf, size_t *out_len);

void nvma_free(uint8_t *buf);

#endif /* NVMA_LIB_H */
