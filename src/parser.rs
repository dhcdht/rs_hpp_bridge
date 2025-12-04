use core::str;

use crate::gen_context::*;

/// 简化类型字符串，移除C++语法如const、&、*等，用于生成合法的函数名
fn simplify_type_for_naming(type_str: &str) -> String {
    // 移除常见的C++修饰符和空格
    let simplified = type_str
        .replace("const ", "")
        .replace("const&", "")
        .replace("&", "")
        .replace("*", "")
        .replace(" ", "")
        .replace("::", "_")
        .replace("<", "_")
        .replace(">", "_")
        .replace(",", "_");
    
    // 如果结果为空或只有下划线，使用默认名称
    if simplified.is_empty() || simplified.chars().all(|c| c == '_') {
        "param".to_string()
    } else {
        simplified
    }
}

pub fn parse_hpp(out_gen_context: &mut GenContext, hpp_path: &str, include_path: &str, cpp_std: &str, extra_clang_args: &[String]) {
    let clang = clang::Clang::new().unwrap();
    let index = clang::Index::new(&clang, true, false);

    // 构建 clang 参数，支持多个 include 路径
    let mut clang_args = vec![
        "-x".to_string(), "c++".to_string(),
        format!("-std={}", cpp_std),
        "-isystem".to_string(), "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/include/c++/v1/".to_string(),
        "-isystem".to_string(), "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/include/".to_string(),
    ];

    // 添加用户指定的 include 路径（支持多个，用冒号分隔）
    for path in include_path.split(':') {
        if !path.is_empty() {
            clang_args.push("-I".to_string());
            clang_args.push(path.to_string());
        }
    }

    // 添加额外的 clang 参数
    clang_args.extend_from_slice(extra_clang_args);

    // 转换为 &str 引用
    let clang_args_refs: Vec<&str> = clang_args.iter().map(|s| s.as_str()).collect();

    let translation_unit = index.parser(hpp_path)
        .arguments(&clang_args_refs)
        .parse().unwrap();

    // 检查 clang 诊断信息，打印错误和致命错误
    let diagnostics = translation_unit.get_diagnostics();
    for diagnostic in &diagnostics {
        use clang::diagnostic::Severity;
        match diagnostic.get_severity() {
            Severity::Fatal | Severity::Error => {
                eprintln!("[clang] {:?}: {}", diagnostic.get_severity(), diagnostic.get_text());
            }
            _ => {} // 忽略警告和提示
        }
    }

    let entity = translation_unit.get_entity();

    let mut file = File::default();
    file.path = entity.get_name().unwrap_or_default();
    let mut file_element = HppElement::File(file);
    visit_parse_clang_entity(&mut file_element, &entity, 0);
    // println!("{:#?}", file_element);

    let mut elements = vec![];
    post_process_hpp_element(out_gen_context, &mut elements, &file_element);
    for element in elements {
        file_element.add_child(element);
    }

    out_gen_context.hpp_elements.push(file_element);
}
// 注意: 这个单元测试已经不维护了
// 主要测试手段是 Flutter 集成测试: tests/flutter_test_project/run_test.sh
// 如需测试，请运行: cd tests/flutter_test_project && ./run_test.sh
#[test]
#[ignore]
fn test_parse_hpp() {
    let mut gen_context = GenContext::default();
    parse_hpp(&mut gen_context, "./tests/parser_test/test.hpp", "./tests/parser_test", "c++20", &[]);
    let result = format!("{:#?}", gen_context);
    let expected = std::fs::read_to_string("./tests/parser_test/ut_result/parse_hpp.txt").unwrap();
    assert_eq!(result, expected);
}

fn visit_parse_clang_entity(out_hpp_element: &mut HppElement, entity: &clang::Entity, indent: usize) {
    if entity.is_in_system_header() {
        return;
    }
    
    // 打开这个可以用来调试查看 clang parser 解析到的数据
    // {
    //     for _ in 0..indent {
    //         print!("  ");
    //     }
    //     println!("{:?}: {}, location={:?}", 
    //         entity.get_kind(), 
    //         entity.get_name().unwrap_or_default(),
    //         entity.get_location(),
    //     );
    // }

    match entity.get_kind() {
        clang::EntityKind::ClassDecl | clang::EntityKind::StructDecl => handle_clang_ClassDecl(out_hpp_element, entity, indent),
        clang::EntityKind::EnumDecl => handle_clang_EnumDecl(out_hpp_element, entity, indent),
        clang::EntityKind::Constructor => handle_clang_Constructor(out_hpp_element, entity, indent),
        clang::EntityKind::Destructor => handle_clang_Destructor(out_hpp_element, entity),
        clang::EntityKind::Method => handle_clang_Method(out_hpp_element, entity, indent),
        clang::EntityKind::ParmDecl => handle_clang_ParmDecl(out_hpp_element, entity),
        clang::EntityKind::FieldDecl => handle_clang_FieldDecl(out_hpp_element, entity, indent),
        // 不属于类的独立函数
        clang::EntityKind::FunctionDecl => handle_clang_FunctionDecl(out_hpp_element, entity, indent),
        _ => {
            for child in entity.get_children() {
                visit_parse_clang_entity(out_hpp_element, &child, indent + 1);
            }
        }
    }
}

fn handle_clang_ClassDecl(out_hpp_element: &mut HppElement, entity: &clang::Entity<'_>, indent: usize) {
    // 跳过系统头文件中的类
    if entity.is_in_system_header() {
        return;
    }

    // 只是前置声明的话，忽略
    if !entity.is_definition() {
        return;
    }
    match out_hpp_element {
        HppElement::File(file) => {
            // 定义不是在当前文件，而是被 inlcude 时，不在这个文件中处理它的桥接生成
            if file.path != entity.get_location().unwrap().get_presumed_location().0 {
                return;
            }
        }
        _ => {
        }
    }
    if let Some(access) = entity.get_accessibility() {
        if access != clang::Accessibility::Public {
            return;
        }
    }
    // 跳过 std namespace 的类
    if let Some(semantic_parent) = entity.get_semantic_parent() {
        if let Some(parent_name) = semantic_parent.get_name() {
            if parent_name.starts_with("__") {
                return; // Skip classes in std namespace
            }
        }
    }
    
    let class_name = entity.get_name().unwrap_or_default();
    let mut class = Class::default();
    class.type_str = class_name.clone();
    {
        // 尝试找出它是不是一个用来回调的类
        // 如果是抽象类
        if entity.is_abstract_record() {
            class.class_type = ClassType::Callback;
        }
        // 注释中有 @callback
        if let Some(comment) = entity.get_comment() {
            if comment.contains("@callback") {
                class.class_type = ClassType::Callback;
            }
        }
        // 类名中有 Callback
        if class_name.contains("Callback") {
            class.class_type = ClassType::Callback;
        }
    }
    class.comment_str = entity.get_comment();
    class.souce_file_path = entity.get_location().unwrap().get_presumed_location().0;
    let mut element = HppElement::Class(class);
    for child in entity.get_children() {
        visit_parse_clang_entity(&mut element, &child, indent + 1);
    }
    // 确保 class 必须有构造和析构函数
    element.ensure_constructor();
    element.ensure_destructor();
    
    out_hpp_element.add_child(element);

    // 为每个类生成对应的 StdPtr class
    let stdptr_element = HppElement::new_stdptr_class_element(class_name);
    out_hpp_element.add_child(stdptr_element);
}

fn post_process_hpp_element(out_gen_context: &mut GenContext, out_hpp_elements: &mut Vec<HppElement>, cur_hpp_element: &HppElement) {
    match cur_hpp_element {
        HppElement::File(file) => {
            for child in &file.children {
                post_process_hpp_element(out_gen_context, out_hpp_elements, child);
            }
        }
        HppElement::Class(class) => {
            for child in &class.children {
                post_process_hpp_element(out_gen_context, out_hpp_elements, child);
            }
        }
        HppElement::Method(method) => {
            // 当用到了某个类型的 std::vector 时，需要生成这个 std::vector 类对应的方法
            if method.return_type.type_kind == TypeKind::StdVector {
                let stdvector_element = HppElement::new_stdvector_class_element(&method.return_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdvector_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdvector_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdvector_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
            // 处理 std::map 返回类型
            else if method.return_type.type_kind == TypeKind::StdMap {
                let stdmap_element = HppElement::new_stdmap_class_element(&method.return_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdmap_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdmap_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdmap_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
            // 处理 std::unordered_map 返回类型
            else if method.return_type.type_kind == TypeKind::StdUnorderedMap {
                let stdunorderedmap_element = HppElement::new_stdunorderedmap_class_element(&method.return_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdunorderedmap_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdunorderedmap_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdunorderedmap_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
            // 处理 std::set 返回类型
            else if method.return_type.type_kind == TypeKind::StdSet {
                let stdset_element = HppElement::new_stdset_class_element(&method.return_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdset_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdset_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdset_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
            // 处理 std::unordered_set 返回类型
            else if method.return_type.type_kind == TypeKind::StdUnorderedSet {
                let stdunorderedset_element = HppElement::new_stdunorderedset_class_element(&method.return_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdunorderedset_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdunorderedset_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdunorderedset_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
            for param in &method.params {
                if param.field_type.type_kind == TypeKind::StdVector {
                        let stdvector_element = HppElement::new_stdvector_class_element(&param.field_type);
                        let already_exists = out_hpp_elements.iter().any(|element| {
                            match element {
                                HppElement::Class(cls) => {
                                    match &stdvector_element {
                                        HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                        _ => false,
                                    }
                                },
                                _ => false,
                            }
                        });
                        
                        if !already_exists {
                            match &stdvector_element {
                                HppElement::Class(_) => {
                                    out_hpp_elements.push(stdvector_element);
                                }
                                _ffi => {
                                    unimplemented!("post_process_hpp_element unimplemented");
                                }
                            }
                        }
                }
                // 处理 std::map 参数类型
                else if param.field_type.type_kind == TypeKind::StdMap {
                        let stdmap_element = HppElement::new_stdmap_class_element(&param.field_type);
                        let already_exists = out_hpp_elements.iter().any(|element| {
                            match element {
                                HppElement::Class(cls) => {
                                    match &stdmap_element {
                                        HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                        _ => false,
                                    }
                                },
                                _ => false,
                            }
                        });
                        
                        if !already_exists {
                            match &stdmap_element {
                                HppElement::Class(_) => {
                                    out_hpp_elements.push(stdmap_element);
                                }
                                _ffi => {
                                    unimplemented!("post_process_hpp_element unimplemented");
                                }
                            }
                        }
                }
                // 处理 std::unordered_map 参数类型
                else if param.field_type.type_kind == TypeKind::StdUnorderedMap {
                        let stdunorderedmap_element = HppElement::new_stdunorderedmap_class_element(&param.field_type);
                        let already_exists = out_hpp_elements.iter().any(|element| {
                            match element {
                                HppElement::Class(cls) => {
                                    match &stdunorderedmap_element {
                                        HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                        _ => false,
                                    }
                                },
                                _ => false,
                            }
                        });
                        
                        if !already_exists {
                            match &stdunorderedmap_element {
                                HppElement::Class(_) => {
                                    out_hpp_elements.push(stdunorderedmap_element);
                                }
                                _ffi => {
                                    unimplemented!("post_process_hpp_element unimplemented");
                                }
                            }
                        }
                }
                // 处理 std::set 参数类型
                else if param.field_type.type_kind == TypeKind::StdSet {
                        let stdset_element = HppElement::new_stdset_class_element(&param.field_type);
                        let already_exists = out_hpp_elements.iter().any(|element| {
                            match element {
                                HppElement::Class(cls) => {
                                    match &stdset_element {
                                        HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                        _ => false,
                                    }
                                },
                                _ => false,
                            }
                        });
                        
                        if !already_exists {
                            match &stdset_element {
                                HppElement::Class(_) => {
                                    out_hpp_elements.push(stdset_element);
                                }
                                _ffi => {
                                    unimplemented!("post_process_hpp_element unimplemented");
                                }
                            }
                        }
                }
                // 处理 std::unordered_set 参数类型
                else if param.field_type.type_kind == TypeKind::StdUnorderedSet {
                        let stdunorderedset_element = HppElement::new_stdunorderedset_class_element(&param.field_type);
                        let already_exists = out_hpp_elements.iter().any(|element| {
                            match element {
                                HppElement::Class(cls) => {
                                    match &stdunorderedset_element {
                                        HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                        _ => false,
                                    }
                                },
                                _ => false,
                            }
                        });
                        
                        if !already_exists {
                            match &stdunorderedset_element {
                                HppElement::Class(_) => {
                                    out_hpp_elements.push(stdunorderedset_element);
                                }
                                _ffi => {
                                    unimplemented!("post_process_hpp_element unimplemented");
                                }
                            }
                        }
                }
            }
        }
        HppElement::Field(field) => {
            if field.field_type.type_kind == TypeKind::StdVector {
                let stdvector_element = HppElement::new_stdvector_class_element(&field.field_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdvector_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdvector_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdvector_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
            // 处理 std::map 字段类型
            else if field.field_type.type_kind == TypeKind::StdMap {
                let stdmap_element = HppElement::new_stdmap_class_element(&field.field_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdmap_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdmap_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdmap_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
            // 处理 std::unordered_map 字段类型
            else if field.field_type.type_kind == TypeKind::StdUnorderedMap {
                let stdunorderedmap_element = HppElement::new_stdunorderedmap_class_element(&field.field_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdunorderedmap_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdunorderedmap_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdunorderedmap_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
            // 处理 std::set 字段类型
            else if field.field_type.type_kind == TypeKind::StdSet {
                let stdset_element = HppElement::new_stdset_class_element(&field.field_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdset_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdset_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdset_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
            // 处理 std::unordered_set 字段类型
            else if field.field_type.type_kind == TypeKind::StdUnorderedSet {
                let stdunorderedset_element = HppElement::new_stdunorderedset_class_element(&field.field_type);
                let already_exists = out_hpp_elements.iter().any(|element| {
                    match element {
                        HppElement::Class(cls) => {
                            match &stdunorderedset_element {
                                HppElement::Class(new_cls) => cls.type_str == new_cls.type_str,
                                _ => false,
                            }
                        },
                        _ => false,
                    }
                });
                
                if !already_exists {
                    match &stdunorderedset_element {
                        HppElement::Class(_) => {
                            out_hpp_elements.push(stdunorderedset_element);
                        }
                        _ffi => {
                            unimplemented!("post_process_hpp_element unimplemented");
                        }
                    }
                }
            }
        }
        _ => {
            // do nothing
        }
    }
}

fn handle_clang_Constructor(out_hpp_element: &mut HppElement, entity: &clang::Entity<'_>, indent: usize) {
    match out_hpp_element {
        HppElement::Class(class) => {
            let mut element = HppElement::Method(Method::default());
            for child in entity.get_arguments().unwrap_or_default() {
                visit_parse_clang_entity(&mut element, &child, indent + 1);
            }
        
            if let HppElement::Method(ref mut updated_method) = element {
                let mut method_name = format!("Constructor");
                for param in &updated_method.params {
                    // 简化类型字符串，移除C++语法如const、&、*等
                    let simplified_type = simplify_type_for_naming(&param.field_type.type_str);
                    method_name.push_str(&format!("_{}", simplified_type));
                }
                updated_method.comment_str = entity.get_comment();
                updated_method.method_type = MethodType::Constructor;
                updated_method.name = method_name;
                updated_method.return_type = FieldType {
                    full_str: format!("{} *", class.type_str),
                    type_str: class.type_str.clone(),
                    type_kind: TypeKind::Class,
                    ptr_level: 1,
                    ..Default::default()
                };
            }
            out_hpp_element.add_child(element);
        }
        _ => {
            // clang 解析出现问题时，构造函数可能出现在非预期的父元素下
            // 这通常是因为头文件包含错误或依赖缺失
            // （不打印详细信息以避免输出过多）
        }
    }
}

fn handle_clang_Destructor(out_hpp_element: &mut HppElement, entity: &clang::Entity,) {
    match out_hpp_element {
        HppElement::Class(class) => {
            let mut method = Method::default();
            method.method_type = MethodType::Destructor;
            method.name = "Destructor".to_string();
            method.return_type = FieldType::new_void();
            method.comment_str = entity.get_comment();
            let element = HppElement::Method(method);
        
            out_hpp_element.add_child(element);
        }
        _ => {
            // clang 解析出现问题时，析构函数可能出现在非预期的父元素下
            // 这通常是因为头文件包含错误或依赖缺失
            // （不打印详细信息以避免输出过多）
        }
    }
}

fn handle_clang_Method(out_hpp_element: &mut HppElement, entity: &clang::Entity<'_>, indent: usize) {
    // 跳过系统头文件中的方法
    if entity.is_in_system_header() {
        return;
    }

    if let Some(access) = entity.get_accessibility() {
        if access != clang::Accessibility::Public {
            return;
        }
    }
    if let Some(name) = entity.get_name() {
        if name.starts_with("operator") {
            // 说明 entity 是一个重载操作符方法，不 bridge 重载函数
            return;
        }
        // 跳过以下划线开头的方法（通常是内部/私有方法）
        if name.starts_with("_") {
            return;
        }
    }

    // 只处理在目标文件中定义的方法，过滤掉来自 include 的头文件的方法
    if let HppElement::Class(class) = out_hpp_element {
        if let Some(location) = entity.get_location() {
            let method_file_path = location.get_presumed_location().0;
            // 如果方法定义的文件和类定义的文件不同，跳过这个方法
            if class.souce_file_path != method_file_path {
                return;
            }
        }
    }
    // 跳过回调类的非 virtual 方法
    match out_hpp_element {
        HppElement::Class(class) => {
            if class.class_type == ClassType::Callback && !entity.is_virtual_method() {
                // Don't bridge non-virtual methods in callback classes
                return;
            }
        },
        _ => {},
    }

    let mut method = Method::default();
    method.name = entity.get_name().unwrap_or_default();
    method.return_type = FieldType::from_clang_type(&entity.get_result_type());
    method.comment_str = entity.get_comment();
    // 检查是否为静态方法
    method.is_static = entity.is_static_method();

    // todo: dhcdht 跳过 callback 中有返回值的方法，这种情况在 dart 中无法处理
    match out_hpp_element {
        HppElement::Class(class) => {
            if class.class_type == ClassType::Callback && method.return_type.type_kind != TypeKind::Void {
                println!("dart 中不支持 callback 中有返回值的方法，所以跳过这个方法: {}", method.name);
                return;
            }
        },
        _ => {},
    }

    let mut element = HppElement::Method(method);
    for child in entity.get_children() {
        visit_parse_clang_entity(&mut element, &child, indent + 1);
    }
    out_hpp_element.add_child(element);
}

fn handle_clang_ParmDecl(out_hpp_element: &mut HppElement, entity: &clang::Entity<'_>) {
    match out_hpp_element {
        HppElement::Method(method) => {
            let mut param = MethodParam::default();
            param.name = entity.get_name().unwrap_or_default();

            // Debug: 打印参数类型信息（已禁用）
            // if param.name == "headers" {
            //     let clang_type = entity.get_type();
            //     println!("[DEBUG] param '{}': type '{}'", param.name,
            //              clang_type.as_ref().map(|t| t.get_display_name()).unwrap_or_default());
            // }

            param.field_type = FieldType::from_clang_type(&entity.get_type());

            method.params.push(param);
        }
        _ => {
            // clang 解析出现问题时，参数可能出现在非预期的父元素下
            // 这通常是因为头文件包含错误或依赖缺失
            // 我们忽略这些参数，而不是让程序崩溃
            // （不打印详细信息以避免输出过多）
        }
    }
}

fn handle_clang_FieldDecl(out_hpp_element: &mut HppElement, entity: &clang::Entity<'_>, indent: usize) {
    if let Some(access) = entity.get_accessibility() {
        if access != clang::Accessibility::Public {
            return;
        }
    }

    let mut field = Field::default();
    field.name = entity.get_name().unwrap_or_default();
    field.field_type = FieldType::from_clang_type(&entity.get_type());
    field.comment_str = entity.get_comment();
    let mut element = HppElement::Field(field);
    for child in entity.get_children() {
        visit_parse_clang_entity(&mut element, &child, indent + 1);
    }
    out_hpp_element.add_child(element);
}

fn handle_clang_FunctionDecl(out_hpp_element: &mut HppElement, entity: &clang::Entity<'_>, indent: usize) {
    // 跳过系统头文件中的函数
    if entity.is_in_system_header() {
        return;
    }

    // 跳过以下划线开头的函数（通常是内部/私有函数）
    if let Some(name) = entity.get_name() {
        if name.starts_with("_") {
            return;
        }
    }

    // 跳过有命名空间的函数（只处理全局函数或者用户自定义命名空间）
    // 通过检查 semantic parent 是否为 TranslationUnit 来判断是否为全局函数
    if let Some(semantic_parent) = entity.get_semantic_parent() {
        let parent_kind = semantic_parent.get_kind();
        // 如果父级不是 TranslationUnit（全局作用域）且不是 Namespace，说明这是第三方库的函数
        if parent_kind != clang::EntityKind::TranslationUnit
            && parent_kind == clang::EntityKind::Namespace {
            // 如果是命名空间，检查是否为常见的第三方库命名空间
            if let Some(parent_name) = semantic_parent.get_name() {
                // std: 标准库命名空间（如 std::vector）
                // __: 编译器内部命名空间（如 __gnu_cxx）
                // detail: 第三方库实现细节命名空间（如 nlohmann::detail，很多 C++ 库用 detail 命名空间存放内部实现）
                // 空字符串: 某些匿名命名空间
                if parent_name.starts_with("std")
                    || parent_name.starts_with("__")
                    || parent_name.contains("detail")
                    || parent_name.len() == 0 {
                    return;
                }
            }
        }
    }

    // 只处理在当前文件中定义的函数
    match out_hpp_element {
        HppElement::File(file) => {
            if file.path != entity.get_location().unwrap().get_presumed_location().0 {
                return;
            }
        }
        _ => {}
    }

    let mut method = Method::default();
    method.name = entity.get_name().unwrap_or_default();
    method.return_type = FieldType::from_clang_type(&entity.get_result_type());
    method.comment_str = entity.get_comment();

    let mut element = HppElement::Method(method);
    for child in entity.get_children() {
        visit_parse_clang_entity(&mut element, &child, indent + 1);
    }
    out_hpp_element.add_child(element);
}

fn handle_clang_EnumDecl(out_hpp_element: &mut HppElement, entity: &clang::Entity<'_>, _indent: usize) {
    // 如果是前向声明，跳过
    if !entity.is_definition() {
        return;
    }

    // 定义不是在当前文件，而是被 include 时，不在这个文件中处理它的桥接生成
    match out_hpp_element {
        HppElement::File(file) => {
            if file.path != entity.get_location().unwrap().get_presumed_location().0 {
                return;
            }
        }
        _ => {
        }
    }

    let name = entity.get_name().unwrap_or_default();
    if name.is_empty() {
        return; // 匿名 enum，跳过
    }

    // 检查是否为 enum class（scoped enum）
    let is_scoped = entity.is_scoped();

    let mut values = Vec::new();

    // 遍历 enum 的子节点，提取枚举值
    for child in entity.get_children() {
        if child.get_kind() == clang::EntityKind::EnumConstantDecl {
            if let Some(const_name) = child.get_name() {
                // 获取枚举值的整数值
                let value = child.get_enum_constant_value().map(|(val, _)| val).unwrap_or(0);
                values.push((const_name, value));
            }
        }
    }

    let enum_def = Enum {
        name,
        is_scoped,
        values,
        comment_str: entity.get_comment(),
    };

    out_hpp_element.add_child(HppElement::Enum(enum_def));
}
