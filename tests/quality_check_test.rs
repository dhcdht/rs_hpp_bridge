/// 代码生成质量检查测试
/// 这些测试确保生成的代码干净、没有第三方库污染
use std::fs;
use std::path::Path;

/// 读取生成的文件内容
fn read_generated_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("无法读取文件: {}", path))
}

/// 测试：生成的 FFI 头文件不应该包含第三方库的内部类型
#[test]
fn test_no_third_party_types_in_ffi_header() {
    // 这个测试需要先运行代码生成
    // 在实际使用中，应该在测试前先调用代码生成
    let test_output = "tests/flutter_test_project/output/test_ffi.h";

    if !Path::new(test_output).exists() {
        eprintln!("跳过测试: {} 不存在，请先运行 ./run_test.sh", test_output);
        return;
    }

    let ffi_header = read_generated_file(test_output);

    // 黑名单：不应该出现的第三方库内部类型
    let forbidden_types = vec![
        // STL 内部类型
        "FFI_iterator",
        "FFI_const_iterator",
        "FFI_reverse_iterator",
        "FFI_const_reverse_iterator",
        "FFI_reference",
        "FFI_const_reference",
        "FFI_pointer",
        "FFI_const_pointer",

        // JSON 库内部类型
        "FFI_json_pointer",
        "FFI_basic_json",
        "FFI_json_sax",

        // 基础类型变体（应该被规范化）
        "FFI_longlong",
        "FFI_unsignedlonglong",

        // 泛型模板参数
        "FFI_T",
        "FFI_U",
        "FFI_V",
    ];

    let mut found_forbidden = Vec::new();
    for forbidden in &forbidden_types {
        // 使用单词边界匹配，避免误匹配（如 FFI_T 匹配到 FFI_TestClass）
        // 检查 typedef 声明中的完整类型名
        let typedef_pattern = format!("typedef void* {};", forbidden);
        if ffi_header.contains(&typedef_pattern) {
            found_forbidden.push(forbidden.to_string());
        }
    }

    assert!(
        found_forbidden.is_empty(),
        "❌ 生成的 FFI 头文件包含不应该存在的类型:\n  - {}\n\n这些类型来自第三方库，应该被过滤掉。",
        found_forbidden.join("\n  - ")
    );
}

/// 测试：生成的 Dart FFI API 不应该包含 ffi__ 开头的函数（无类名的第三方库函数）
#[test]
fn test_no_ffi_double_underscore_in_dart_ffiapi() {
    let test_output = "tests/flutter_test_project/output/test_ffiapi.dart";

    if !Path::new(test_output).exists() {
        eprintln!("跳过测试: {} 不存在", test_output);
        return;
    }

    let ffiapi = read_generated_file(test_output);

    // 黑名单：不应该出现的 ffi__ 函数（第三方库的方法）
    let forbidden_patterns = vec![
        // JSON 库方法
        "ffi__has_subtype",
        "ffi__clear_subtype",
        "ffi__null()",
        "ffi__boolean",
        "ffi__end_object",
        "ffi__end_array",
        "ffi__is_errored",
        "ffi__skip_bom",
        "ffi__skip_whitespace",
        "ffi__accept",
        "ffi__pop_back",
        "ffi__empty",
        "ffi__is_primitive",
    ];

    let mut found_forbidden = Vec::new();
    for pattern in &forbidden_patterns {
        if ffiapi.contains(pattern) {
            found_forbidden.push(pattern.to_string());
        }
    }

    assert!(
        found_forbidden.is_empty(),
        "❌ 生成的 Dart FFI API 包含不应该存在的函数:\n  - {}\n\n这些函数来自第三方库（如 nlohmann json），应该被过滤掉。",
        found_forbidden.join("\n  - ")
    );
}

/// 测试：生成的 Dart 类文件不应该包含第三方库的方法
#[test]
fn test_no_third_party_methods_in_dart_classes() {
    let test_output = "tests/flutter_test_project/output/test.dart";

    if !Path::new(test_output).exists() {
        eprintln!("跳过测试: {} 不存在", test_output);
        return;
    }

    let dart_class = read_generated_file(test_output);

    // 黑名单：不应该出现的第三方库方法
    let forbidden_methods = vec![
        "has_subtype()",
        "clear_subtype()",
        "null()",  // 注意：这是方法调用，不是空值检查
        "boolean(",
        "skip_bom()",
        "skip_whitespace()",
        "is_errored()",
    ];

    let mut found_forbidden = Vec::new();
    for method in &forbidden_methods {
        if dart_class.contains(method) {
            found_forbidden.push(method.to_string());
        }
    }

    assert!(
        found_forbidden.is_empty(),
        "❌ 生成的 Dart 类包含不应该存在的方法:\n  - {}\n\n这些方法来自第三方库（如 nlohmann json），应该被过滤掉。",
        found_forbidden.join("\n  - ")
    );
}

/// 测试：typedef 数量应该在合理范围内
#[test]
fn test_typedef_count_reasonable() {
    let test_output = "tests/flutter_test_project/output/test_ffi.h";

    if !Path::new(test_output).exists() {
        eprintln!("跳过测试: {} 不存在", test_output);
        return;
    }

    let ffi_header = read_generated_file(test_output);

    // 统计 typedef 数量
    let typedef_count = ffi_header.matches("typedef void* FFI_").count();

    // 对于测试项目（test.hpp），预期的 typedef 数量应该在合理范围
    // 太多可能意味着包含了不该有的类型
    // 太少可能意味着过滤过度
    let min_expected = 5;   // 至少应该有几个业务类型
    let max_expected = 50;  // 不应该超过这个数量

    assert!(
        typedef_count >= min_expected,
        "❌ typedef 数量太少 ({}), 可能过滤过度。预期至少 {} 个。",
        typedef_count,
        min_expected
    );

    assert!(
        typedef_count <= max_expected,
        "❌ typedef 数量太多 ({}), 可能包含了第三方库类型。预期最多 {} 个。\n\n请检查是否有不该生成的类型。",
        typedef_count,
        max_expected
    );

    println!("✓ typedef 数量合理: {} 个（范围 {}-{}）", typedef_count, min_expected, max_expected);
}

/// 测试：确保业务类型被正确生成（白名单）
#[test]
fn test_expected_business_types_present() {
    let test_output = "tests/flutter_test_project/output/test_ffi.h";

    if !Path::new(test_output).exists() {
        eprintln!("跳过测试: {} 不存在", test_output);
        return;
    }

    let ffi_header = read_generated_file(test_output);

    // 白名单：应该存在的业务类型
    let expected_types = vec![
        "FFI_TestClass",
        "FFI_MyCallback",
        "FFI_SimpleStruct",
        "FFI_StdPtr_TestClass",
        "FFI_StdPtr_MyCallback",
        "FFI_StdVector_int",
        "FFI_StdVector_float",
    ];

    let mut missing_types = Vec::new();
    for expected in &expected_types {
        if !ffi_header.contains(expected) {
            missing_types.push(expected.to_string());
        }
    }

    assert!(
        missing_types.is_empty(),
        "❌ 生成的 FFI 头文件缺少预期的业务类型:\n  - {}\n\n这些类型应该被生成但未找到。",
        missing_types.join("\n  - ")
    );

    println!("✓ 所有预期的业务类型都已生成");
}
