# The Flutter tooling requires that developers have CMake 3.10 or later
# installed. You should not increase this version, as doing so will cause
# the plugin to fail to compile for some customers of the plugin.
cmake_minimum_required(VERSION 3.10)

project(flutter_test_project_library VERSION 0.0.1 LANGUAGES C)

add_library(flutter_test_project SHARED
  "flutter_test_project.c"
  "test.cpp"
  "person.cpp"
  "company.cpp"
  "simple_a.cpp"
  "simple_b.cpp"
  # RsHppBridge 生成的文件
  "output/test_ffi.cpp"
  "output/simple_types_ffi.cpp"
  "output/simple_a_ffi.cpp"
  "output/simple_b_ffi.cpp"
)

set_target_properties(flutter_test_project PROPERTIES
  PUBLIC_HEADER "flutter_test_project.h test.hpp test_ffi.h base_types.hpp person.hpp company.hpp simple_types.hpp simple_a.hpp simple_b.hpp output/simple_types_ffi.h output/simple_a_ffi.h output/simple_b_ffi.h"
  OUTPUT_NAME "flutter_test_project"
)

target_compile_definitions(flutter_test_project PUBLIC DART_SHARED_LIB)

if (ANDROID)
  # Support Android 15 16k page size
  target_link_options(flutter_test_project PRIVATE "-Wl,-z,max-page-size=16384")
endif()
