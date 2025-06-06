// Relative import to be able to reuse the C sources.
// See the comment in ../flutter_test_project.podspec for more information.
extern "C" {
#include "../../src/flutter_test_project.c"
#include "../../src/dart_sdk_include/dart_api_dl.c"
}
#include "../../src/test.cpp"
#include "../../src/simple_a.cpp"
#include "../../src/simple_b.cpp"
#include "../../src/simple_types.hpp"

#include "../../src/output/test_ffi.cpp"
#include "../../src/output/simple_a_ffi.cpp"
#include "../../src/output/simple_b_ffi.cpp"
#include "../../src/output/simple_types_ffi.cpp"
