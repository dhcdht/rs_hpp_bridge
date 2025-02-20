use core::str;

use crate::gen_context::*;

pub fn parse_hpp(out_gen_context: &mut GenContext, hpp_path: &str) {
    let clang = clang::Clang::new().unwrap();
    let index = clang::Index::new(&clang, true, false);
    let translation_unit = index.parser(hpp_path)
        .arguments(&[
            "-x", "c++", 
            "-isystem", "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/include/c++/v1/",
            "-isystem", "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/include/",
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
    parse_hpp(&mut gen_context, "./tests/1/test.hpp");
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
    if let Some(access) = entity.get_accessibility() {
        if (access != clang::Accessibility::Public) {
            return;
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
                if !out_hpp_elements.contains(&stdvector_element) {
                    match &stdvector_element {
                        HppElement::Class(class) => {
                            out_hpp_elements.push(stdvector_element);
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
                        if !out_hpp_elements.contains(&stdvector_element) {
                            match &stdvector_element {
                                HppElement::Class(class) => {
                                    out_hpp_elements.push(stdvector_element);
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
                if !out_hpp_elements.contains(&stdvector_element) {
                    match &stdvector_element {
                        HppElement::Class(class) => {
                            out_hpp_elements.push(stdvector_element);
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
                    method_name.push_str(&format!("_{}", param.field_type.type_str));
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
