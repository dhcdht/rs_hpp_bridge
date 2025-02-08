use core::{fmt, str};
use std::fmt::Debug;

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

#[derive(Debug, Default)]
pub struct GenContext {
    pub hpp_elements: Vec<HppElement>,
    pub class_names: Vec<String>,
}

pub enum HppElement {
    File(File),
    Class(Class),
    Method(Method),
    Field(Field),
}

#[derive(Debug, Default)]
pub struct File {
    pub path: String,

    pub children: Vec<HppElement>,
}

#[derive(Debug, Default)]
pub struct Class {
    pub type_str: String,

    pub children: Vec<HppElement>,
}

#[derive(Debug, Default)]
pub struct Method {
    pub name: String,
    pub return_type_str: String,
    pub params: Vec<MethodParam>,
}

#[derive(Debug, Default)]
pub struct Field {
    pub name: String,
}

#[derive(Debug, Default)]
pub struct MethodParam {
    pub name: String,
    pub type_str: String,
}

impl HppElement {
    fn add_child(&mut self, child: HppElement) {
        match self {
            HppElement::File(file) => {
                file.children.push(child);
            },
            HppElement::Class(class) => {
                class.children.push(child);
            }
            _ => {

            },
        }
    }
}

impl fmt::Debug for HppElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File(arg0) => arg0.fmt(f),
            Self::Class(arg0) => arg0.fmt(f),
            Self::Method(arg0) => arg0.fmt(f),
            Self::Field(arg0) => arg0.fmt(f),
        }
    }
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
        clang::EntityKind::Method => 'block: {
            if entity.get_accessibility().unwrap() != clang::Accessibility::Public {
                break 'block;
            }

            let mut method = Method::default();
            method.name = entity.get_name().unwrap_or_default();
            method.return_type_str = entity.get_result_type().unwrap().get_display_name();

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
                    param.type_str = entity.get_type().unwrap().get_display_name();

                    method.params.push(param);
                }
                _ => {

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
            method.return_type_str = entity.get_result_type().unwrap().get_display_name();

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

        }
    }
}
