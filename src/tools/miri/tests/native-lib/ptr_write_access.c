#include <stddef.h>

// See comments in build_native_lib()
#define EXPORT __attribute__((visibility("default")))

/* Test: test_modify_int */

EXPORT void modify_int(int *ptr) {
  *ptr += 1;
}

/* Test: test_init_int */

EXPORT void init_int(int *ptr) {
  *ptr = 12;
}

/* Test: test_init_array */

EXPORT void init_array(int *array, size_t len, int value) {
  for (size_t i = 0; i < len; i++) {
    array[i] = value;
  }
}

/* Test: test_swap_ptr */

EXPORT void swap_ptr(const int **pptr0, const int **pptr1) {
  const int *tmp = *pptr0;
  *pptr0 = *pptr1;
  *pptr1 = tmp;
}

/* Test: test_init_static_inner */

EXPORT void init_static_inner(int **const pptr) {
  **pptr = 1234;
}

/* Test: test_dangling */

EXPORT void write_nullptr(const int **pptr) {
  *pptr = NULL;
}