use core::{fmt, str};
use std::fmt::Debug;

pub fn parse_hpp(hpp_path: &str) -> HppElement {
    println!("parse_hpp: {}", hpp_path);

    let clang = clang::Clang::new().unwrap();
    let index = clang::Index::new(&clang, false, false);
    let translation_unit = index.parser(hpp_path).parse().unwrap();
    let entity = translation_unit.get_entity();

    let mut file = File::default();
    file.path = entity.get_name().unwrap_or_default();
    let mut ret = HppElement::File(file);
    visit_entity(&mut ret, &entity, 0);
    println!("{:#?}", ret);

    return ret;
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
    pub typeStr: String,

    pub children: Vec<HppElement>,
}

#[derive(Debug, Default)]
pub struct Method {
    pub name: String,
    pub returnTypeStr: String,
    pub params: Vec<MethodParam>,
}

#[derive(Debug, Default)]
pub struct Field {
    pub name: String,
}

#[derive(Debug, Default)]
pub struct MethodParam {
    pub name: String,
    pub typeStr: String,
}

impl HppElement {
    fn addChild(&mut self, child: HppElement) {
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

fn visit_entity(ret: &mut HppElement, entity: &clang::Entity, indent: usize) {
    // for _ in 0..indent {
    //     print!(" ");
    // }
    // println!("{:?}: {}", 
    //     entity.get_kind(), 
    //     entity.get_name().unwrap_or_default(),
    // );

    match entity.get_kind() {
        clang::EntityKind::ClassDecl => {
            let mut class = Class::default();
            class.typeStr = entity.get_name().unwrap_or_default();

            let mut element = HppElement::Class(class);
            for child in entity.get_children() {
                visit_entity(&mut element, &child, indent + 1);
            }
            ret.addChild(element);
        }
        clang::EntityKind::Method => 'block: {
            if entity.get_accessibility().unwrap() != clang::Accessibility::Public {
                break 'block;
            }

            let mut method = Method::default();
            method.name = entity.get_name().unwrap_or_default();
            method.returnTypeStr = entity.get_result_type().unwrap().get_display_name();

            let mut element = HppElement::Method(method);
            for child in entity.get_children() {
                visit_entity(&mut element, &child, indent + 1);
            }
            ret.addChild(element);
        }
        clang::EntityKind::ParmDecl => {
            match ret {
                HppElement::Method(method) => {
                    let mut param = MethodParam::default();
                    param.name = entity.get_name().unwrap_or_default();
                    param.typeStr = entity.get_type().unwrap().get_display_name();

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
                visit_entity(&mut element, &child, indent + 1);
            }
            ret.addChild(element);
        }
        _ => {
            for child in entity.get_children() {
                visit_entity(ret, &child, indent + 1);
            }
        }
    }
}
