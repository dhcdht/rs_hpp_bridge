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

#[derive(Debug, Default, PartialEq, Eq)]
pub enum ClassType {
    #[default]
    Normal,
    Callback,
    StdPtr,
}

#[derive(Debug, Default)]
pub struct Class {
    pub type_str: String,
    pub class_type: ClassType,

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
    pub field_type: FieldType,
}

#[derive(Debug, Default)]
pub struct MethodParam {
    pub name: String,
    pub field_type: FieldType,
}

/// 类型的种类
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum TypeKind {
    #[default]
    Void,
    Int64,
    Float,
    Double,
    Char,

    String,

    Class,
    StdPtr,
}

/// 返回值、字段、参数等的类型
#[derive(Debug, Default, PartialEq, Eq, Clone)]
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

    /// 确保 class 必须有构造函数
    pub fn ensure_constructor(&mut self) {
        match self {
            HppElement::Class(class) => {
                for child in &class.children {
                    if let HppElement::Method(method) = child {
                        if (method.method_type == MethodType::Constructor) {
                            return;
                        }
                    }
                }

                let mut method = Method::default();
                method.method_type = MethodType::Constructor;
                method.name = "Constructor".to_string();
                method.return_type = FieldType {
                    full_str: format!("{} *", class.type_str),
                    type_str: class.type_str.clone(),
                    type_kind: TypeKind::Class,
                    ptr_level: 1,
                };
                let element = HppElement::Method(method);
                class.children.push(element);
            }
            _ => {
                unimplemented!("HppElement::ensure_constructor, {:?}", self);
            },
        }
    }

    /// 确保 class 必须有析构函数
    pub fn ensure_destructor(&mut self) {
        match self {
            HppElement::Class(class) => {
                for child in &class.children {
                    if let HppElement::Method(method) = child {
                        if (method.method_type == MethodType::Destructor) {
                            return;
                        }
                    }
                }

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
                class.children.push(element);
            }
            _ => {
                unimplemented!("HppElement::ensure_destructor, {:?}", self);
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
        return self.class_type == ClassType::Callback
    }
}

pub trait OptionClassExt {
    fn get_class_name_or_empty(&self) -> &str;
}
impl OptionClassExt for Option<&Class> {
    fn get_class_name_or_empty(&self) -> &str {
        match self {
            Some(class) => class.type_str.as_str(),
            None => "",
        }
    }
}

impl Method {
    pub fn new_get_for_field(field: &Field) -> Self {
        return Method {
            method_type: MethodType::Normal,
            name: format!("get_{}", field.name),
            return_type: field.field_type.clone(),
            params: vec![],
        };
    }
    pub fn new_set_for_field(field: &Field) -> Self {
        return Method {
            method_type: MethodType::Normal,
            name: format!("set_{}", field.name),
            return_type: FieldType::new_void(),
            params: vec![MethodParam {
                name: field.name.clone(),
                field_type: field.field_type.clone(),
            }],
        };
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
        // std::shared_ptr
        else if lower_full_str.starts_with("std::shared_ptr") {
            field_type.type_kind = TypeKind::StdPtr;
            if let (Some(start), Some(end)) = (field_type.full_str.find('<'), field_type.full_str.rfind('>')) {
                field_type.type_str = field_type.full_str[start + 1..end].trim().to_string();
            } else {
                field_type.type_str = "std::shared_ptr".to_string();
            }
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

    pub fn new_void() -> Self {
        return FieldType {
            full_str: "void".to_string(),
            type_str: "void".to_string(),
            type_kind: TypeKind::Void,
            ptr_level: 0,
        };
    }
}
