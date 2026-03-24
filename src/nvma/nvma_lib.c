#include "nvma_lib.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <ctype.h>
#include <stdint.h>

/* ── Syscall table ─────────────────────────────────────────────── */

typedef struct {
    const char *name;
    int number;
} Syscall;

static const Syscall syscalls[] = {
    {"exit",        0x00},
    {"spawn",       0x01},
    {"open",        0x02},
    {"read",        0x03},
    {"write",       0x04},
    {"create",      0x05},
    {"delete",      0x06},
    {"cap_request", 0x07},
    {"cap_spawn",   0x08},
    {"drv_call",    0x09},
    {"msg_send",    0x0A},
    {"msg_recieve", 0x0B},
    {"inb",         0x0C},
    {"outb",        0x0D},
    {"print",       0x0E},
    {NULL, 0}
};

typedef struct {
    char    *name;
    uint32_t address;
} Label;


typedef struct {
    Label   *labels;
    int      label_count;
    uint32_t current_address;

    uint8_t *buf;
    size_t   buf_cap;
    size_t   buf_len;
} nvma_ctx_t;

/* ── Helpers ───────────────────────────────────────────────────── */

static int find_syscall(const char *name) {
    for (int i = 0; syscalls[i].name != NULL; i++) {
        if (strcmp(syscalls[i].name, name) == 0)
            return syscalls[i].number;
    }
    return -1;
}

static uint32_t parse_number(const char *str) {
    if (str[0] == '\'' && str[1] != '\0' && str[2] == '\'')
        return (uint32_t)str[1];
    if (strncmp(str, "0x", 2) == 0 || strncmp(str, "0X", 2) == 0)
        return (uint32_t)strtoul(str + 2, NULL, 16);
    return (uint32_t)atoi(str);
}

static void ctx_add_label(nvma_ctx_t *ctx, const char *name, uint32_t addr) {
    ctx->labels = (Label *)realloc(ctx->labels,
                                   (ctx->label_count + 1) * sizeof(Label));
    ctx->labels[ctx->label_count].name    = strdup(name);
    ctx->labels[ctx->label_count].address = addr;
    ctx->label_count++;
}

static int ctx_find_label(nvma_ctx_t *ctx, const char *name, uint32_t *addr) {
    for (int i = 0; i < ctx->label_count; i++) {
        if (strcmp(ctx->labels[i].name, name) == 0) {
            *addr = ctx->labels[i].address;
            return 1;
        }
    }
    return 0;
}

static void ctx_free_labels(nvma_ctx_t *ctx) {
    for (int i = 0; i < ctx->label_count; i++)
        free(ctx->labels[i].name);
    free(ctx->labels);
    ctx->labels      = NULL;
    ctx->label_count = 0;
}

/* ── Output buffer helpers ─────────────────────────────────────── */

static void ctx_ensure(nvma_ctx_t *ctx, size_t extra) {
    while (ctx->buf_len + extra > ctx->buf_cap) {
        ctx->buf_cap = ctx->buf_cap ? ctx->buf_cap * 2 : 256;
        ctx->buf = (uint8_t *)realloc(ctx->buf, ctx->buf_cap);
    }
}

static void ctx_emit_byte(nvma_ctx_t *ctx, uint8_t b) {
    ctx_ensure(ctx, 1);
    ctx->buf[ctx->buf_len++] = b;
}

static void ctx_emit_u32(nvma_ctx_t *ctx, uint32_t v) {
    ctx_emit_byte(ctx, (v >> 24) & 0xFF);
    ctx_emit_byte(ctx, (v >> 16) & 0xFF);
    ctx_emit_byte(ctx, (v >>  8) & 0xFF);
    ctx_emit_byte(ctx,  v        & 0xFF);
}

/* ── Instruction size (first pass) ─────────────────────────────── */

#ifdef _WIN32
#define CASECMP _stricmp
#else
#define CASECMP strcasecmp
#endif

static int get_instruction_size(char *tokens[], int token_count) {
    if (token_count == 0) return 0;
    const char *mn = tokens[0];

    if (CASECMP(mn, ".NVM0") == 0)                         return 0;
    /* basic */
    if (CASECMP(mn, "hlt") == 0 || CASECMP(mn, "halt") == 0) return 1;
    if (CASECMP(mn, "nop") == 0)                            return 1;
    if (CASECMP(mn, "push") == 0 && token_count >= 2)       return 5;
    if (CASECMP(mn, "pop") == 0)                            return 1;
    if (CASECMP(mn, "dup") == 0)                            return 1;
    if (CASECMP(mn, "swap") == 0)                           return 1;
    /* arithmetic */
    if (CASECMP(mn, "add") == 0)                            return 1;
    if (CASECMP(mn, "sub") == 0)                            return 1;
    if (CASECMP(mn, "mul") == 0)                            return 1;
    if (CASECMP(mn, "div") == 0)                            return 1;
    if (CASECMP(mn, "mod") == 0)                            return 1;
    /* comparison */
    if (CASECMP(mn, "cmp") == 0)                            return 1;
    if (CASECMP(mn, "eq") == 0)                             return 1;
    if (CASECMP(mn, "neq") == 0)                            return 1;
    if (CASECMP(mn, "gt") == 0)                             return 1;
    if (CASECMP(mn, "lt") == 0)                             return 1;
    /* flow control */
    if (CASECMP(mn, "jmp") == 0)                            return 5;
    if (CASECMP(mn, "jz") == 0)                             return 5;
    if (CASECMP(mn, "jnz") == 0)                            return 5;
    if (CASECMP(mn, "call") == 0)                           return 5;
    if (CASECMP(mn, "ret") == 0)                            return 1;
    /* frame instructions (new) */
    if (CASECMP(mn, "enter") == 0)                          return 2;
    if (CASECMP(mn, "leave") == 0)                          return 1;
    if (CASECMP(mn, "load_arg") == 0)                       return 2;
    if (CASECMP(mn, "store_arg") == 0)                      return 2;
    if (CASECMP(mn, "load_rel") == 0)                       return 2;
    if (CASECMP(mn, "store_rel") == 0)                      return 2;
    /* memory */
    if (CASECMP(mn, "load") == 0)                           return 2;
    if (CASECMP(mn, "store") == 0)                          return 2;
    if (CASECMP(mn, "load_abs") == 0)                       return 1;
    if (CASECMP(mn, "store_abs") == 0)                      return 1;
    /* system */
    if (CASECMP(mn, "syscall") == 0)                        return 2;
    if (CASECMP(mn, "break") == 0)                          return 1;

    return 0;
}

typedef struct {
    const char *text;
    size_t      len;
    size_t      pos;
} LineIter;

static int line_next(LineIter *it, char *dst, size_t dst_size) {
    if (it->pos >= it->len) return 0;

    size_t start = it->pos;
    while (it->pos < it->len && it->text[it->pos] != '\n')
        it->pos++;

    size_t line_len = it->pos - start;
    /* skip \r if present */
    size_t copy_len = line_len;
    if (copy_len > 0 && it->text[start + copy_len - 1] == '\r')
        copy_len--;

    if (copy_len >= dst_size)
        copy_len = dst_size - 1;
    memcpy(dst, it->text + start, copy_len);
    dst[copy_len] = '\0';

    /* skip the newline */
    if (it->pos < it->len) it->pos++;
    return 1;
}

static void line_reset(LineIter *it) {
    it->pos = 0;
}

/* ── Trim + strip comments helper ──────────────────────────────── */

/* Returns pointer into buf (in-place), modifies buf. */
static char *prepare_line(char *buf) {
    /* remove comments */
    char *comment = strchr(buf, ';');
    if (comment) *comment = '\0';

    /* trim leading whitespace */
    char *start = buf;
    while (isspace((unsigned char)*start)) start++;

    /* trim trailing whitespace */
    size_t slen = strlen(start);
    if (slen > 0) {
        char *end = start + slen - 1;
        while (end > start && isspace((unsigned char)*end)) end--;
        end[1] = '\0';
    }
    return start;
}

/* ── Tokenize helper ───────────────────────────────────────────── */

static int tokenize(char *line, char *tokens[], int max_tokens) {
    int count = 0;
    char *tok = strtok(line, " \t,");
    while (tok && count < max_tokens) {
        tokens[count++] = tok;
        tok = strtok(NULL, " \t,");
    }
    return count;
}

/* ── Main assembly function ────────────────────────────────────── */

int nvma_assemble(const char *asm_text, size_t asm_len,
                  uint8_t **out_buf, size_t *out_len)
{
    if (!asm_text || !out_buf || !out_len) return -1;

    nvma_ctx_t ctx;
    memset(&ctx, 0, sizeof(ctx));
    ctx.current_address = 4; /* after NVM0 signature */

    LineIter iter = { asm_text, asm_len, 0 };
    char line[512];

    /* ── First pass: collect labels ──────────────────────────────── */
    while (line_next(&iter, line, sizeof(line))) {
        char *start = prepare_line(line);
        if (strlen(start) == 0) continue;

        /* label? */
        size_t slen = strlen(start);
        if (start[slen - 1] == ':') {
            char label_name[256];
            size_t llen = slen - 1;
            if (llen >= sizeof(label_name)) llen = sizeof(label_name) - 1;
            memcpy(label_name, start, llen);
            label_name[llen] = '\0';
            ctx_add_label(&ctx, label_name, ctx.current_address);
            continue;
        }

        /* tokenize to compute size */
        char linecopy[512];
        strncpy(linecopy, start, sizeof(linecopy) - 1);
        linecopy[sizeof(linecopy) - 1] = '\0';

        char *tokens[4];
        int tc = tokenize(linecopy, tokens, 4);
        ctx.current_address += get_instruction_size(tokens, tc);
    }

    /* ── Second pass: emit bytecode ──────────────────────────────── */
    line_reset(&iter);
    ctx.current_address = 4;

    /* NVM0 signature */
    ctx_emit_byte(&ctx, 0x4E); /* N */
    ctx_emit_byte(&ctx, 0x56); /* V */
    ctx_emit_byte(&ctx, 0x4D); /* M */
    ctx_emit_byte(&ctx, 0x30); /* 0 */

    while (line_next(&iter, line, sizeof(line))) {
        char *start = prepare_line(line);
        if (strlen(start) == 0) continue;

        /* skip labels */
        if (start[strlen(start) - 1] == ':') continue;

        char linecopy[512];
        strncpy(linecopy, start, sizeof(linecopy) - 1);
        linecopy[sizeof(linecopy) - 1] = '\0';

        char *tokens[4];
        int tc = tokenize(linecopy, tokens, 4);
        if (tc == 0) continue;

        const char *mn = tokens[0];

        if (CASECMP(mn, ".NVM0") == 0) {
            /* signature already emitted */
        }
        /* ── basic ─────────────────────────────────────────────── */
        else if (CASECMP(mn, "hlt") == 0 || CASECMP(mn, "halt") == 0) {
            ctx_emit_byte(&ctx, 0x00);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "nop") == 0) {
            ctx_emit_byte(&ctx, 0x01);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "push") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x02);
            uint32_t val = parse_number(tokens[1]);
            ctx_emit_u32(&ctx, val);
            ctx.current_address += 5;
        }
        else if (CASECMP(mn, "pop") == 0) {
            ctx_emit_byte(&ctx, 0x04);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "dup") == 0) {
            ctx_emit_byte(&ctx, 0x05);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "swap") == 0) {
            ctx_emit_byte(&ctx, 0x06);
            ctx.current_address += 1;
        }
        /* ── arithmetic ────────────────────────────────────────── */
        else if (CASECMP(mn, "add") == 0) {
            ctx_emit_byte(&ctx, 0x10);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "sub") == 0) {
            ctx_emit_byte(&ctx, 0x11);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "mul") == 0) {
            ctx_emit_byte(&ctx, 0x12);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "div") == 0) {
            ctx_emit_byte(&ctx, 0x13);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "mod") == 0) {
            ctx_emit_byte(&ctx, 0x14);
            ctx.current_address += 1;
        }
        /* ── comparison ────────────────────────────────────────── */
        else if (CASECMP(mn, "cmp") == 0) {
            ctx_emit_byte(&ctx, 0x20);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "eq") == 0) {
            ctx_emit_byte(&ctx, 0x21);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "neq") == 0) {
            ctx_emit_byte(&ctx, 0x22);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "gt") == 0) {
            ctx_emit_byte(&ctx, 0x23);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "lt") == 0) {
            ctx_emit_byte(&ctx, 0x24);
            ctx.current_address += 1;
        }
        /* ── flow control ──────────────────────────────────────── */
        else if (CASECMP(mn, "jmp") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x30);
            uint32_t addr;
            if (!ctx_find_label(&ctx, tokens[1], &addr))
                addr = parse_number(tokens[1]);
            ctx_emit_u32(&ctx, addr);
            ctx.current_address += 5;
        }
        else if (CASECMP(mn, "jz") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x31);
            uint32_t addr;
            if (!ctx_find_label(&ctx, tokens[1], &addr))
                addr = parse_number(tokens[1]);
            ctx_emit_u32(&ctx, addr);
            ctx.current_address += 5;
        }
        else if (CASECMP(mn, "jnz") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x32);
            uint32_t addr;
            if (!ctx_find_label(&ctx, tokens[1], &addr))
                addr = parse_number(tokens[1]);
            ctx_emit_u32(&ctx, addr);
            ctx.current_address += 5;
        }
        else if (CASECMP(mn, "call") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x33);
            uint32_t addr;
            if (!ctx_find_label(&ctx, tokens[1], &addr))
                addr = parse_number(tokens[1]);
            ctx_emit_u32(&ctx, addr);
            ctx.current_address += 5;
        }
        else if (CASECMP(mn, "ret") == 0) {
            ctx_emit_byte(&ctx, 0x34);
            ctx.current_address += 1;
        }
        /* ── frame instructions (new) ──────────────────────────── */
        else if (CASECMP(mn, "enter") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x35);
            ctx_emit_byte(&ctx, (uint8_t)parse_number(tokens[1]));
            ctx.current_address += 2;
        }
        else if (CASECMP(mn, "leave") == 0) {
            ctx_emit_byte(&ctx, 0x36);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "load_arg") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x37);
            ctx_emit_byte(&ctx, (uint8_t)parse_number(tokens[1]));
            ctx.current_address += 2;
        }
        else if (CASECMP(mn, "store_arg") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x38);
            ctx_emit_byte(&ctx, (uint8_t)parse_number(tokens[1]));
            ctx.current_address += 2;
        }
        else if (CASECMP(mn, "load_rel") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x42);
            ctx_emit_byte(&ctx, (uint8_t)parse_number(tokens[1]));
            ctx.current_address += 2;
        }
        else if (CASECMP(mn, "store_rel") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x43);
            ctx_emit_byte(&ctx, (uint8_t)parse_number(tokens[1]));
            ctx.current_address += 2;
        }
        /* ── memory ────────────────────────────────────────────── */
        else if (CASECMP(mn, "load") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x40);
            ctx_emit_byte(&ctx, (uint8_t)parse_number(tokens[1]));
            ctx.current_address += 2;
        }
        else if (CASECMP(mn, "store") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x41);
            ctx_emit_byte(&ctx, (uint8_t)parse_number(tokens[1]));
            ctx.current_address += 2;
        }
        else if (CASECMP(mn, "load_abs") == 0) {
            ctx_emit_byte(&ctx, 0x44);
            ctx.current_address += 1;
        }
        else if (CASECMP(mn, "store_abs") == 0) {
            ctx_emit_byte(&ctx, 0x45);
            ctx.current_address += 1;
        }
        /* ── system ────────────────────────────────────────────── */
        else if (CASECMP(mn, "syscall") == 0 && tc >= 2) {
            ctx_emit_byte(&ctx, 0x50);
            int sc = find_syscall(tokens[1]);
            if (sc == -1) sc = (int)parse_number(tokens[1]);
            ctx_emit_byte(&ctx, (uint8_t)(sc & 0xFF));
            ctx.current_address += 2;
        }
        else if (CASECMP(mn, "break") == 0) {
            ctx_emit_byte(&ctx, 0x51);
            ctx.current_address += 1;
        }
        /* unknown instruction — skip silently */
    }

    ctx_free_labels(&ctx);

    *out_buf = ctx.buf;
    *out_len = ctx.buf_len;
    return 0;
}

void nvma_free(uint8_t *buf) {
    free(buf);
}
