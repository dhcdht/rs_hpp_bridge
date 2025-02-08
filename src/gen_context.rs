use core::fmt;

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

#[derive(Debug, Default, PartialEq, Eq)]
pub enum MethodType {
    /// 实例方法
    #[default]
    Normal,
    /// 构造函数
    Constructor,
    /// 析构函数
    Destructor,
}

#[derive(Debug, Default)]
pub struct Method {
    pub method_type: MethodType,
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
    pub fn add_child(&mut self, child: HppElement) {
        match self {
            HppElement::File(file) => {
                file.children.push(child);
            },
            HppElement::Class(class) => {
                class.children.push(child);
            }
            _ => {
                unimplemented!("HppElement::add_child");
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
