#include <stddef.h>

// See comments in build_native_lib()
#define EXPORT __attribute__((visibility("default")))

/* Test: test_modify_int */

EXPORT void modify_int(int *ptr) {
  *ptr += 2;
}

/* Test: test_init_int */

EXPORT void init_int(int *ptr) {
  *ptr = 29;
}

/* Test: test_init_array */

EXPORT void init_array(int *array, size_t len, int value) {
  for (size_t i = 0; i < len; i++) {
    array[i] = value;
  }
}

/* Test: test_swap_ptr */

EXPORT void swap_ptr(const int **x, const int **y) {
  const int *tmp = *x;
  *x = *y;
  *y = tmp;
}