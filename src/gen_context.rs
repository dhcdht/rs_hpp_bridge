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
    pub return_type: FieldType,
    pub params: Vec<MethodParam>,
}

#[derive(Debug, Default)]
pub struct Field {
    pub name: String,
}

#[derive(Debug, Default)]
pub struct MethodParam {
    pub name: String,
    pub field_type: FieldType,
}

/// 类型的种类
#[derive(Debug, Default, PartialEq, Eq)]
pub enum TypeKind {
    #[default]
    Void,
    Int64,
    Float,
    Double,
    Class,
}

/// 返回值、字段、参数等的类型
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FieldType {
    /// 类型字符串，包含全修饰的类型名
    pub full_str: String,
    /// 只是类型名，不包含修饰
    pub type_str: String,
    pub type_kind: TypeKind,
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

impl FieldType {
    pub fn from_clang_type(clang_type: &Option<clang::Type>) -> Self {
        let mut field_type = FieldType::default();
        field_type.full_str = clang_type.unwrap().get_display_name();
        match field_type.full_str.to_lowercase().as_str() {
            "void" => {
                field_type.type_kind = TypeKind::Void;
                field_type.type_str = "void".to_string();
            }
            "int" => {
                field_type.type_kind = TypeKind::Int64;
                field_type.type_str = "int".to_string();
            }
            "float" => {
                field_type.type_kind = TypeKind::Float;
                field_type.type_str = "float".to_string();
            }
            "double" => {
                field_type.type_kind = TypeKind::Double;
                field_type.type_str = "double".to_string();
            }
            _ => {
                field_type.type_kind = TypeKind::Class;
                field_type.type_str = clang_type.unwrap().get_pointee_type().unwrap().get_display_name();
            }
            
        }

        return field_type;
    }
}
