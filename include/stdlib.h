#ifndef _NVM_STDLIB_H
#define _NVM_STDLIB_H

__attribute__((optnone))
static int abs(int x) {
    if (x < 0) return -x;
    return x;
}

__attribute__((optnone))
static int min(int a, int b) {
    if (a < b) return a;
    return b;
}

__attribute__((optnone))
static int max(int a, int b) {
    if (a > b) return a;
    return b;
}

#endif
