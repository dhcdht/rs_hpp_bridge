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

pub fn parse_hpp(out_gen_context: &mut GenContext, hpp_path: &str, include_path: &str) {
    let clang = clang::Clang::new().unwrap();
    let index = clang::Index::new(&clang, true, false);
    let translation_unit = index.parser(hpp_path)
        .arguments(&[
            "-x", "c++", 
            "-isystem", "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/include/c++/v1/",
            "-isystem", "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/include/",
            "-isystem", include_path,
        ])
        .parse().unwrap();
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
#[test]
fn test_parse_hpp() {
    let mut gen_context = GenContext::default();
    parse_hpp(&mut gen_context, "./tests/1/test.hpp", "");
    let result = format!("{:#?}", gen_context);
    let expected = std::fs::read_to_string("./tests/1/ut_result/parse_hpp.txt").unwrap();
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
        if (access != clang::Accessibility::Public) {
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
            if (method.return_type.type_kind == TypeKind::StdVector) {
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
            else if (method.return_type.type_kind == TypeKind::StdMap) {
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
            else if (method.return_type.type_kind == TypeKind::StdUnorderedMap) {
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
            else if (method.return_type.type_kind == TypeKind::StdSet) {
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
            else if (method.return_type.type_kind == TypeKind::StdUnorderedSet) {
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
                if (param.field_type.type_kind == TypeKind::StdVector) {
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
                else if (param.field_type.type_kind == TypeKind::StdMap) {
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
                else if (param.field_type.type_kind == TypeKind::StdUnorderedMap) {
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
                else if (param.field_type.type_kind == TypeKind::StdSet) {
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
                else if (param.field_type.type_kind == TypeKind::StdUnorderedSet) {
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
            unimplemented!("clang::EntityKind::Constructor");
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
            unimplemented!("clang::EntityKind::Destructor")
        }
    }
}

fn handle_clang_Method(out_hpp_element: &mut HppElement, entity: &clang::Entity<'_>, indent: usize) {
    if let Some(access) = entity.get_accessibility() {
        if (access != clang::Accessibility::Public) {
            return;
        }
    }
    if let Some(name) = entity.get_name() {
        if name.starts_with("operator") {
            // 说明 entity 是一个重载操作符方法，不 bridge 重载函数
            return;
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
            param.field_type = FieldType::from_clang_type(&entity.get_type());

            method.params.push(param);
        }
        _ => {
            unimplemented!("clang::EntityKind::ParmDecl");
        }
    }
}

fn handle_clang_FieldDecl(out_hpp_element: &mut HppElement, entity: &clang::Entity<'_>, indent: usize) {
    if let Some(access) = entity.get_accessibility() {
        if (access != clang::Accessibility::Public) {
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
