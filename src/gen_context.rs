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

    /// 是否是抽象类，也就是用于 callback 的类
    is_abstract: bool,
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
    Char,

    String,

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
    /// 几级指针，0 T, 1 T*, 2 T**
    pub ptr_level: i32,
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

impl Class {
    pub fn is_callback(&self) -> bool {
        self.is_abstract
    }

    pub fn set_is_abstract(&mut self, is_abstract: bool) {
        self.is_abstract = is_abstract;
    }
}

impl FieldType {
    pub fn from_clang_type(clang_type: &Option<clang::Type>) -> Self {
        let mut field_type = FieldType::default();
        field_type.full_str = clang_type.unwrap().get_display_name();
        let lower_full_str = field_type.full_str.to_lowercase();

        // 一些特殊处理的类型
        // std::string
        if lower_full_str == "std::string" || lower_full_str == "string" {
            field_type.type_kind = TypeKind::String;
            field_type.full_str = "std::string".to_string();
            field_type.type_str = "std::string".to_string();
            return field_type;
        }

        // 计算指针级别
        let ptr_level = lower_full_str.chars().rev().take_while(|&c| c == '*').count();
        field_type.ptr_level = ptr_level as i32;
        
        let lower_full_str_without_ptr = lower_full_str.trim_end_matches('*').trim();
        match lower_full_str_without_ptr {
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
            "char" => {
                field_type.type_kind = TypeKind::Char;
                field_type.type_str = "char".to_string();
            }
            _ => {
                // 指针类型
                if let Some(pointee) = clang_type.unwrap().get_pointee_type() {
                    field_type.type_kind = TypeKind::Class;
                    field_type.type_str = clang_type.unwrap().get_pointee_type().unwrap().get_display_name();
                }
                // 非指针类型 
                else {
                    field_type.type_kind = TypeKind::Class;
                    field_type.type_str = clang_type.unwrap().get_display_name();
                }
            }
            
        }

        return field_type;
    }
}
