#ifndef LIBCSV_TEST_COMMON_H
#define LIBCSV_TEST_COMMON_H

#include <csv.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void fail_assertion(const char *file, int line, const char *expr) {
    fprintf(stderr, "%s:%d: assertion failed: %s\n", file, line, expr);
    exit(EXIT_FAILURE);
}

static void fail_size_eq(
    const char *file,
    int line,
    const char *actual_expr,
    size_t actual,
    const char *expected_expr,
    size_t expected
) {
    fprintf(
        stderr,
        "%s:%d: expected %s == %s, got %lu and %lu\n",
        file,
        line,
        actual_expr,
        expected_expr,
        (unsigned long)actual,
        (unsigned long)expected
    );
    exit(EXIT_FAILURE);
}

static void fail_int_eq(
    const char *file,
    int line,
    const char *actual_expr,
    int actual,
    const char *expected_expr,
    int expected
) {
    fprintf(
        stderr,
        "%s:%d: expected %s == %s, got %d and %d\n",
        file,
        line,
        actual_expr,
        expected_expr,
        actual,
        expected
    );
    exit(EXIT_FAILURE);
}

static void fail_bytes_eq(
    const char *file,
    int line,
    const char *actual_expr,
    const unsigned char *actual,
    const char *expected_expr,
    const unsigned char *expected,
    size_t len
) {
    size_t i;

    fprintf(
        stderr,
        "%s:%d: expected %s == %s for %lu bytes\n",
        file,
        line,
        actual_expr,
        expected_expr,
        (unsigned long)len
    );
    fprintf(stderr, "actual:");
    for (i = 0; i < len; i++) {
        fprintf(stderr, " %02x", actual[i]);
    }
    fprintf(stderr, "\nexpected:");
    for (i = 0; i < len; i++) {
        fprintf(stderr, " %02x", expected[i]);
    }
    fprintf(stderr, "\n");
    exit(EXIT_FAILURE);
}

#define ASSERT(expr)                                                          \
    do {                                                                      \
        if (!(expr)) {                                                        \
            fail_assertion(__FILE__, __LINE__, #expr);                        \
        }                                                                     \
    } while (0)

#define ASSERT_SIZE_EQ(actual, expected)                                      \
    do {                                                                      \
        size_t actual_value = (actual);                                       \
        size_t expected_value = (expected);                                   \
        if (actual_value != expected_value) {                                 \
            fail_size_eq(                                                     \
                __FILE__,                                                     \
                __LINE__,                                                     \
                #actual,                                                      \
                actual_value,                                                 \
                #expected,                                                    \
                expected_value                                                \
            );                                                                \
        }                                                                     \
    } while (0)

#define ASSERT_INT_EQ(actual, expected)                                       \
    do {                                                                      \
        int actual_value = (actual);                                          \
        int expected_value = (expected);                                      \
        if (actual_value != expected_value) {                                 \
            fail_int_eq(                                                      \
                __FILE__,                                                     \
                __LINE__,                                                     \
                #actual,                                                      \
                actual_value,                                                 \
                #expected,                                                    \
                expected_value                                                \
            );                                                                \
        }                                                                     \
    } while (0)

#define ASSERT_BYTES_EQ(actual, expected, len)                                \
    do {                                                                      \
        const unsigned char *actual_value =                                   \
            (const unsigned char *)(actual);                                  \
        const unsigned char *expected_value =                                 \
            (const unsigned char *)(expected);                                \
        size_t expected_len = (len);                                          \
        if (memcmp(actual_value, expected_value, expected_len) != 0) {        \
            fail_bytes_eq(                                                    \
                __FILE__,                                                     \
                __LINE__,                                                     \
                #actual,                                                      \
                actual_value,                                                 \
                #expected,                                                    \
                expected_value,                                               \
                expected_len                                                  \
            );                                                                \
        }                                                                     \
    } while (0)

#endif
