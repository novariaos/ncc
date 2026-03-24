#ifndef _NVM_STRING_H
#define _NVM_STRING_H

static int strlen(const char *s) {
    int len = 0;
    while (s[len] != '\0') {
        len++;
    }
    return len;
}

static int strcmp(const char *a, const char *b) {
    int i = 0;
    while (a[i] != '\0' && a[i] == b[i]) {
        i++;
    }
    return a[i] - b[i];
}

#endif
