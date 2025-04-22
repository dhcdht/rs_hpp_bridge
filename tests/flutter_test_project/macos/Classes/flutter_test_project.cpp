// Relative import to be able to reuse the C sources.
// See the comment in ../flutter_test_project.podspec for more information.
extern "C" {
#include "../../src/flutter_test_project.c"
#include "../../src/dart_sdk_include/dart_api_dl.c"
}
#include "../../src/test.cpp"
#include "../../src/test_ffi.cpp"
