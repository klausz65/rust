#include <stddef.h>

// See comments in build_native_lib()
#define EXPORT __attribute__((visibility("default")))

/* Test: test_modify_int */

EXPORT void modify_int(int *ptr) {
  *ptr += 1;
}

/* Test: test_init_int */

EXPORT void init_int(int *ptr) {
  *ptr = 21;
}

/* Test: test_init_array */

EXPORT void init_array(int *array, size_t len) {
  for (size_t i = 0; i < len; i++) {
    array[i] = 31;
  }
}

/* Test: test_swap_ptr */

EXPORT void swap_ptr(const int **pptr0, const int **pptr1) {
  const int *tmp = *pptr0;
  *pptr0 = *pptr1;
  *pptr1 = tmp;
}

/* Test: test_init_interior_mutable */

typedef struct UnsafeInterior {
    int *mut_ptr;
} UnsafeInterior;

EXPORT void init_interior_mutable(const UnsafeInterior *u_ptr) {
  *(u_ptr->mut_ptr) = 51;
}

/* Test: test_dangling */

EXPORT void overwrite_ptr(const int **pptr) {
  *pptr = NULL;
}
