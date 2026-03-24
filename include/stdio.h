#ifndef _NVM_STDIO_H
#define _NVM_STDIO_H

#include "nvm.h"

static int putchar(int ch) {
    __nvm_write(__nvm_tty_fd(), ch);
    return ch;
}

static void print_char(int ch) {
    __nvm_write(__nvm_tty_fd(), ch);
}

static void print_int(int n) {
    if (n < 0) {
        putchar('-');
        n = -n;
    }
    if (n >= 10) {
        print_int(n / 10);
    }
    putchar('0' + n % 10);
}

static void print_ln(void) {
    putchar('\n');
}

int printf(const char *fmt, ...);

#endif
