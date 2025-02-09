use core::str;

use crate::gen_context::*;

pub fn parse_hpp(out_gen_context: &mut GenContext, hpp_path: &str) {
    let clang = clang::Clang::new().unwrap();
    let index = clang::Index::new(&clang, false, false);
    let translation_unit = index.parser(hpp_path).parse().unwrap();
    let entity = translation_unit.get_entity();

    let mut file = File::default();
    file.path = entity.get_name().unwrap_or_default();
    let mut file_element = HppElement::File(file);
    visit_parse_clang_entity(&mut file_element, &entity, 0);
    // println!("{:#?}", file_element);

    visit_parse_hpp_element(out_gen_context, &file_element);
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
    // 打开这个可以用来调试查看 clang parser 解析到的数据
    // {
    //     for _ in 0..indent {
    //         print!("  ");
    //     }
    //     println!("{:?}: {}", 
    //         entity.get_kind(), 
    //         entity.get_name().unwrap_or_default(),
    //     );
    // }

    match entity.get_kind() {
        clang::EntityKind::ClassDecl => {
            let mut class = Class::default();
            class.type_str = entity.get_name().unwrap_or_default();

            let mut element = HppElement::Class(class);
            for child in entity.get_children() {
                visit_parse_clang_entity(&mut element, &child, indent + 1);
            }
            out_hpp_element.add_child(element);
        }
        clang::EntityKind::Constructor => 'block: {
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
                        updated_method.method_type = MethodType::Constructor;
                        updated_method.name = method_name;
                        updated_method.return_type = FieldType {
                            full_str: format!("{} *", class.type_str),
                            type_str: class.type_str.clone(),
                            type_kind: TypeKind::Class,
                            ptr_level: 1,
                        };
                    }
                    out_hpp_element.add_child(element);
                }
                _ => {
                    unimplemented!("clang::EntityKind::Constructor");
                }
            }
        }
        clang::EntityKind::Destructor => 'block: {
            match out_hpp_element {
                HppElement::Class(class) => {
                    let mut method = Method::default();
                    method.method_type = MethodType::Destructor;
                    method.name = "Destructor".to_string();
                    method.return_type = FieldType {
                        full_str: "void".to_string(),
                        type_str: "void".to_string(),
                        type_kind: TypeKind::Void,
                        ptr_level: 0,
                    };
                    let element = HppElement::Method(method);
                    
                    out_hpp_element.add_child(element);
                }
                _ => {
                    unimplemented!("clang::EntityKind::Destructor")
                }
            }
        }
        clang::EntityKind::Method => 'block: {
            if entity.get_accessibility().unwrap() != clang::Accessibility::Public {
                break 'block;
            }

            let mut method = Method::default();
            method.name = entity.get_name().unwrap_or_default();
            method.return_type = FieldType::from_clang_type(&entity.get_result_type());

            let mut element = HppElement::Method(method);
            for child in entity.get_children() {
                visit_parse_clang_entity(&mut element, &child, indent + 1);
            }
            out_hpp_element.add_child(element);
        }
        clang::EntityKind::ParmDecl => {
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
        clang::EntityKind::FieldDecl => 'block: {
            if entity.get_accessibility().unwrap() != clang::Accessibility::Public {
                break 'block;
            }

            let mut field = Field::default();
            field.name = entity.get_name().unwrap_or_default();

            let mut element = HppElement::Field(field);
            for child in entity.get_children() {
                visit_parse_clang_entity(&mut element, &child, indent + 1);
            }
            out_hpp_element.add_child(element);
        }
        // 不属于类的独立函数
        clang::EntityKind::FunctionDecl => 'block: {
            let mut method = Method::default();
            method.name = entity.get_name().unwrap_or_default();
            method.return_type = FieldType::from_clang_type(&entity.get_result_type());

            let mut element = HppElement::Method(method);
            for child in entity.get_children() {
                visit_parse_clang_entity(&mut element, &child, indent + 1);
            }
            out_hpp_element.add_child(element);
        }
        _ => {
            for child in entity.get_children() {
                visit_parse_clang_entity(out_hpp_element, &child, indent + 1);
            }
        }
    }
}

fn visit_parse_hpp_element(out_gen_context: &mut GenContext, hpp_element: &HppElement) {
    match hpp_element {
        HppElement::File(file) => {
            for child in &file.children {
                visit_parse_hpp_element(out_gen_context, child);
            }
        }
        HppElement::Class(class) => {
            out_gen_context.class_names.push(class.type_str.clone());
        }
        _ => {
            // do nothing
        }
    }
}
