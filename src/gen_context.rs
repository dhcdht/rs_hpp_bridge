use core::fmt;

/// 检查类型名是否应该被忽略（不生成绑定）
/// 这些类型包括：模板参数、第三方库内部类型、STL 内部类型等
pub fn should_ignore_type(type_str: &str) -> bool {
    let lower = type_str.to_lowercase();

    // 过滤模板参数（单字母或短名称）
    if type_str.len() <= 2 && type_str.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return true;
    }

    // 过滤包含 "typename" 的类型（模板内部类型）
    if lower.contains("typename") {
        return true;
    }

    // 过滤包含模板语法的类型（如 <IteratorType>）
    if type_str.contains("<") || type_str.contains(">") {
        return true;
    }

    // 过滤以特定后缀结尾的类型（第三方库类型别名）
    let filtered_suffixes = [
        "_type", "_t", "_ptr", "_opt_t", "_u",
    ];
    for suffix in &filtered_suffixes {
        if lower.ends_with(suffix) {
            return true;
        }
    }

    // 过滤包含 "error" 后缀的异常类型（如 other_error, type_error, parse_error）
    if lower.ends_with("error") || lower.ends_with("_error") {
        return true;
    }

    // 过滤可能是模板类型参数的通用名称
    let generic_type_params = [
        "containertype", "chartype", "valuetype", "keytype",
        "iteratortype", "elementtype", "itemtype",
        // STL 迭代器和内部类型
        "iterator", "const_iterator", "reverse_iterator", "const_reverse_iterator",
        "reference", "const_reference", "pointer", "const_pointer",
        // JSON 库内部类型
        "json_pointer",
    ];
    for param in &generic_type_params {
        if lower == *param {
            return true;
        }
    }

    // 过滤第三方库类型关键字（使用更精确的匹配）
    let third_party_patterns = [
        "nlohmann", "basicjson",  // nlohmann json 库的内部类型
        "sockaddr", "structsockaddr",  // socket 结构体
        "eventloop", "hssl", "reconn", "unpack",  // libhv 库类型
        "tcpsocket", "udpsocket", "tsocketchannel",
        "cbor_", "json_sax", "parser_callback",  // JSON 库的特定内部类型
        "timerid", "buffer",  // libhv 的其他类型
        "out_of_range",  // 标准异常类型
    ];

    for pattern in &third_party_patterns {
        if lower.contains(pattern) {
            return true;
        }
    }

    // 过滤以 "_" 开头或结尾的类型（通常是内部类型）
    if type_str.starts_with("_") || type_str.ends_with("_") {
        return true;
    }

    // 过滤包含 "detail" 的类型（第三方库内部实现）
    if lower.contains("detail") {
        return true;
    }

    // 过滤特定的常见 C/C++ 基础类型（unsigned、long 变体）
    let basic_type_variants = [
        "unsignedint", "unsignedlong", "longlong",
        "unsignedlonglong", "unsignedchar", "unsignedshort",
    ];

    for variant in &basic_type_variants {
        if lower == *variant || lower.contains(variant) {
            return true;
        }
    }

    false
}

#[derive(Debug, Default)]
pub struct GenContext {
    pub module_name: String,
    pub hpp_elements: Vec<HppElement>,
}

#[derive(PartialEq, Eq)]
pub enum HppElement {
    File(File),
    Class(Class),
    Method(Method),
    Field(Field),
    Enum(Enum),
}

#[derive(Debug, Default, PartialEq, Eq)]
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
    StdVector,
    StdMap,
    StdUnorderedMap,
    StdSet,
    StdUnorderedSet,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Class {
    pub type_str: String,
    pub class_type: ClassType,

    pub children: Vec<HppElement>,

    /// 如果是模板类型，这里存储模板参数
    pub value_type: Option<Box<FieldType>>,
    /// 如果是 map 类型，这里存储 key 类型
    pub key_type: Option<Box<FieldType>>,
    /// 注释
    pub comment_str: Option<String>,
    /// 源文件位置
    pub souce_file_path: String,
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

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Method {
    pub method_type: MethodType,
    pub name: String,
    pub return_type: FieldType,
    pub params: Vec<MethodParam>,
    /// 是否为静态方法
    pub is_static: bool,
    /// 注释
    pub comment_str: Option<String>,
    /// callback 是否在原线程同步调用（用于回调类的方法）
    /// true = 同步调用（使用函数指针）
    /// false = 异步调用（使用 SendPort，默认）
    pub is_sync_callback: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,

    /// 注释
    pub comment_str: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Enum {
    pub name: String,
    /// true = enum class (scoped), false = 普通 enum
    pub is_scoped: bool,
    /// 枚举值列表：(名称, 数值)
    pub values: Vec<(String, i64)>,
    /// 注释
    pub comment_str: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
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
    Bool,

    String,

    Class,
    Enum,
    StdPtr,
    StdVector,
    StdMap,
    StdUnorderedMap,
    StdSet,
    StdUnorderedSet,

    /// 应该被忽略的类型（模板参数、第三方库内部类型等）
    Ignored,
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

    /// 如果是模板类型，这里存储模板参数
    pub value_type: Option<Box<FieldType>>,
    /// 如果是 map 类型，这里存储 key 类型
    pub key_type: Option<Box<FieldType>>,
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
                // clang 解析出现问题时，可能会尝试向非预期的元素添加子元素
                // 这通常是因为头文件包含错误或依赖缺失，忽略即可
            },
        }
    }

    /// 确保 class 必须有构造函数
    pub fn ensure_constructor(&mut self) {
        match self {
            HppElement::Class(class) => {
                for child in &class.children {
                    if let HppElement::Method(method) = child {
                        if method.method_type == MethodType::Constructor {
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
                    ..Default::default()
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
                        if method.method_type == MethodType::Destructor {
                            return;
                        }
                    }
                }

                let mut method = Method::default();
                method.method_type = MethodType::Destructor;
                method.name = "Destructor".to_string();
                method.return_type = FieldType::new_void();
                let element = HppElement::Method(method);
                class.children.push(element);
            }
            _ => {
                unimplemented!("HppElement::ensure_destructor, {:?}", self);
            },
        }
    }

    pub fn new_stdptr_class_element(class_name: String) -> Self {
        let mut stdptr_class = Class::default();
        stdptr_class.type_str = format!("StdPtr_{}", class_name);
        stdptr_class.class_type = ClassType::StdPtr;
        let mut stdptr_element = HppElement::Class(stdptr_class);
        // StdPtr class 的构造函数
        let constructor_method = Method {
            method_type: MethodType::Constructor,
            name: "Constructor".to_string(),
            return_type: FieldType {
                full_str: format!("StdPtr_{}", class_name),
                type_str: class_name.clone(),
                type_kind: TypeKind::StdPtr,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![MethodParam {
                name: "obj".to_string(),
                field_type: FieldType {
                    full_str: format!("{} *", class_name),
                    type_str: class_name.clone(),
                    type_kind: TypeKind::Class,
                    ptr_level: 1,
                    ..Default::default()
                },
            }],
            ..Default::default()
        };
        stdptr_element.add_child(HppElement::Method(constructor_method));
        // StdPtr class 的析构函数
        stdptr_element.ensure_destructor();
        // std::shared_ptr.get()
        let get_ptr_method = Method {
            method_type: MethodType::Normal,
            name: "get".to_string(),
            return_type: FieldType {
                full_str: format!("{} *", class_name),
                type_str: class_name,
                type_kind: TypeKind::Class,
                ptr_level: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        stdptr_element.add_child(HppElement::Method(get_ptr_method));

        return stdptr_element;
    }

    pub fn new_stdvector_class_element(field_type: &FieldType) -> Self {
        let class_name = field_type.get_value_type_str();

        let mut stdvector_class = Class::default();
        stdvector_class.type_str = format!("StdVector_{}", class_name);
        stdvector_class.class_type = ClassType::StdVector;
        stdvector_class.value_type = field_type.value_type.clone();
        let mut stdvector_element = HppElement::Class(stdvector_class);
        // StdVector class 的构造函数
        let constructor_method = Method {
            method_type: MethodType::Constructor,
            name: "Constructor".to_string(),
            return_type: field_type.clone(),
            ..Default::default()
        };
        stdvector_element.add_child(HppElement::Method(constructor_method));
        // StdPtr class 的析构函数
        stdvector_element.ensure_destructor();
        // std::shared_ptr.get()
        let size_method = Method {
            method_type: MethodType::Normal,
            name: "size".to_string(),
            return_type: FieldType {
                full_str: "int".to_string(),
                type_str: "int".to_string(),
                type_kind: TypeKind::Int64,
                ptr_level: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        stdvector_element.add_child(HppElement::Method(size_method));
        let get_method = Method {
            method_type: MethodType::Normal,
            name: "at".to_string(),
            return_type: (**field_type.value_type.as_ref().unwrap()).clone(),
            params: vec![MethodParam {
                name: "index".to_string(),
                field_type: FieldType {
                    full_str: "int".to_string(),
                    type_str: "int".to_string(),
                    type_kind: TypeKind::Int64,
                    ptr_level: 0,
                    ..Default::default()
                },
            }],
            ..Default::default()
        };
        stdvector_element.add_child(HppElement::Method(get_method));

        return stdvector_element;
    }

    pub fn new_stdmap_class_element(field_type: &FieldType) -> Self {
        let key_type_name = field_type.get_key_type_str();
        let value_type_name = field_type.get_value_type_str();

        let mut stdmap_class = Class::default();
        stdmap_class.type_str = format!("StdMap_{}_{}", key_type_name, value_type_name);
        stdmap_class.class_type = ClassType::StdMap;
        stdmap_class.key_type = field_type.key_type.clone();
        stdmap_class.value_type = field_type.value_type.clone();
        let mut stdmap_element = HppElement::Class(stdmap_class);

        // 如果 key_type 或 value_type 解析失败，只创建基本的构造和析构函数
        if field_type.key_type.is_none() || field_type.value_type.is_none() {
            // 构造函数
            let constructor_method = Method {
                method_type: MethodType::Constructor,
                name: "Constructor".to_string(),
                return_type: field_type.clone(),
                ..Default::default()
            };
            stdmap_element.add_child(HppElement::Method(constructor_method));
            stdmap_element.ensure_destructor();
            return stdmap_element;
        }

        // StdMap class 的构造函数
        let constructor_method = Method {
            method_type: MethodType::Constructor,
            name: "Constructor".to_string(),
            return_type: field_type.clone(),
            ..Default::default()
        };
        stdmap_element.add_child(HppElement::Method(constructor_method));
        
        // StdMap class 的析构函数
        stdmap_element.ensure_destructor();
        
        // size 方法
        let size_method = Method {
            method_type: MethodType::Normal,
            name: "size".to_string(),
            return_type: FieldType {
                full_str: "int".to_string(),
                type_str: "int".to_string(),
                type_kind: TypeKind::Int64,
                ptr_level: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        stdmap_element.add_child(HppElement::Method(size_method));
        
        // insert 方法
        let insert_method = Method {
            method_type: MethodType::Normal,
            name: "insert".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "key".to_string(),
                    field_type: (**field_type.key_type.as_ref().unwrap()).clone(),
                },
                MethodParam {
                    name: "value".to_string(),
                    field_type: (**field_type.value_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdmap_element.add_child(HppElement::Method(insert_method));
        
        // find 方法
        let find_method = Method {
            method_type: MethodType::Normal,
            name: "find".to_string(),
            return_type: (**field_type.value_type.as_ref().unwrap()).clone(),
            params: vec![
                MethodParam {
                    name: "key".to_string(),
                    field_type: (**field_type.key_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdmap_element.add_child(HppElement::Method(find_method));
        
        // count 方法 (替代 contains，更兼容)
        let count_method = Method {
            method_type: MethodType::Normal,
            name: "count".to_string(),
            return_type: FieldType {
                full_str: "int".to_string(),
                type_str: "int".to_string(),
                type_kind: TypeKind::Int64,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "key".to_string(),
                    field_type: (**field_type.key_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdmap_element.add_child(HppElement::Method(count_method));
        
        // erase 方法
        let erase_method = Method {
            method_type: MethodType::Normal,
            name: "erase".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "key".to_string(),
                    field_type: (**field_type.key_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdmap_element.add_child(HppElement::Method(erase_method));
        
        // clear 方法
        let clear_method = Method {
            method_type: MethodType::Normal,
            name: "clear".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        stdmap_element.add_child(HppElement::Method(clear_method));

        return stdmap_element;
    }

    pub fn new_stdunorderedmap_class_element(field_type: &FieldType) -> Self {
        let key_type_name = field_type.get_key_type_str();
        let value_type_name = field_type.get_value_type_str();

        let mut stdunorderedmap_class = Class::default();
        stdunorderedmap_class.type_str = format!("StdUnorderedMap_{}_{}", key_type_name, value_type_name);
        stdunorderedmap_class.class_type = ClassType::StdUnorderedMap;
        stdunorderedmap_class.key_type = field_type.key_type.clone();
        stdunorderedmap_class.value_type = field_type.value_type.clone();
        let mut stdunorderedmap_element = HppElement::Class(stdunorderedmap_class);

        // 如果 key_type 或 value_type 解析失败，只创建基本的构造和析构函数
        if field_type.key_type.is_none() || field_type.value_type.is_none() {
            let constructor_method = Method {
                method_type: MethodType::Constructor,
                name: "Constructor".to_string(),
                return_type: field_type.clone(),
                ..Default::default()
            };
            stdunorderedmap_element.add_child(HppElement::Method(constructor_method));
            stdunorderedmap_element.ensure_destructor();
            return stdunorderedmap_element;
        }

        // StdUnorderedMap class 的构造函数
        let constructor_method = Method {
            method_type: MethodType::Constructor,
            name: "Constructor".to_string(),
            return_type: field_type.clone(),
            ..Default::default()
        };
        stdunorderedmap_element.add_child(HppElement::Method(constructor_method));
        
        // StdUnorderedMap class 的析构函数
        stdunorderedmap_element.ensure_destructor();
        
        // size 方法
        let size_method = Method {
            method_type: MethodType::Normal,
            name: "size".to_string(),
            return_type: FieldType {
                full_str: "int".to_string(),
                type_str: "int".to_string(),
                type_kind: TypeKind::Int64,
                ptr_level: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        stdunorderedmap_element.add_child(HppElement::Method(size_method));
        
        // insert 方法
        let insert_method = Method {
            method_type: MethodType::Normal,
            name: "insert".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "key".to_string(),
                    field_type: (**field_type.key_type.as_ref().unwrap()).clone(),
                },
                MethodParam {
                    name: "value".to_string(),
                    field_type: (**field_type.value_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdunorderedmap_element.add_child(HppElement::Method(insert_method));
        
        // find 方法
        let find_method = Method {
            method_type: MethodType::Normal,
            name: "find".to_string(),
            return_type: (**field_type.value_type.as_ref().unwrap()).clone(),
            params: vec![
                MethodParam {
                    name: "key".to_string(),
                    field_type: (**field_type.key_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdunorderedmap_element.add_child(HppElement::Method(find_method));
        
        // count 方法 (替代 contains，更兼容)
        let count_method = Method {
            method_type: MethodType::Normal,
            name: "count".to_string(),
            return_type: FieldType {
                full_str: "int".to_string(),
                type_str: "int".to_string(),
                type_kind: TypeKind::Int64,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "key".to_string(),
                    field_type: (**field_type.key_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdunorderedmap_element.add_child(HppElement::Method(count_method));
        
        // erase 方法
        let erase_method = Method {
            method_type: MethodType::Normal,
            name: "erase".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "key".to_string(),
                    field_type: (**field_type.key_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdunorderedmap_element.add_child(HppElement::Method(erase_method));
        
        // clear 方法
        let clear_method = Method {
            method_type: MethodType::Normal,
            name: "clear".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        stdunorderedmap_element.add_child(HppElement::Method(clear_method));

        return stdunorderedmap_element;
    }

    pub fn new_stdset_class_element(field_type: &FieldType) -> Self {
        let value_type_name = field_type.get_value_type_str();

        let mut stdset_class = Class::default();
        stdset_class.type_str = format!("StdSet_{}", value_type_name);
        stdset_class.class_type = ClassType::StdSet;
        stdset_class.value_type = field_type.value_type.clone();
        let mut stdset_element = HppElement::Class(stdset_class);
        
        // StdSet class 的构造函数
        let constructor_method = Method {
            method_type: MethodType::Constructor,
            name: "Constructor".to_string(),
            return_type: field_type.clone(),
            ..Default::default()
        };
        stdset_element.add_child(HppElement::Method(constructor_method));
        
        // StdSet class 的析构函数
        stdset_element.ensure_destructor();
        
        // size 方法
        let size_method = Method {
            method_type: MethodType::Normal,
            name: "size".to_string(),
            return_type: FieldType {
                full_str: "int".to_string(),
                type_str: "int".to_string(),
                type_kind: TypeKind::Int64,
                ptr_level: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        stdset_element.add_child(HppElement::Method(size_method));
        
        // insert 方法
        let insert_method = Method {
            method_type: MethodType::Normal,
            name: "insert".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "value".to_string(),
                    field_type: (**field_type.value_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdset_element.add_child(HppElement::Method(insert_method));
        
        // count 方法 (替代 contains，更兼容)
        let count_method = Method {
            method_type: MethodType::Normal,
            name: "count".to_string(),
            return_type: FieldType {
                full_str: "int".to_string(),
                type_str: "int".to_string(),
                type_kind: TypeKind::Int64,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "value".to_string(),
                    field_type: (**field_type.value_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdset_element.add_child(HppElement::Method(count_method));
        
        // erase 方法
        let erase_method = Method {
            method_type: MethodType::Normal,
            name: "erase".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "value".to_string(),
                    field_type: (**field_type.value_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdset_element.add_child(HppElement::Method(erase_method));
        
        // clear 方法
        let clear_method = Method {
            method_type: MethodType::Normal,
            name: "clear".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        stdset_element.add_child(HppElement::Method(clear_method));

        return stdset_element;
    }

    pub fn new_stdunorderedset_class_element(field_type: &FieldType) -> Self {
        let value_type_name = field_type.get_value_type_str();

        let mut stdunorderedset_class = Class::default();
        stdunorderedset_class.type_str = format!("StdUnorderedSet_{}", value_type_name);
        stdunorderedset_class.class_type = ClassType::StdUnorderedSet;
        stdunorderedset_class.value_type = field_type.value_type.clone();
        let mut stdunorderedset_element = HppElement::Class(stdunorderedset_class);
        
        // StdUnorderedSet class 的构造函数
        let constructor_method = Method {
            method_type: MethodType::Constructor,
            name: "Constructor".to_string(),
            return_type: field_type.clone(),
            ..Default::default()
        };
        stdunorderedset_element.add_child(HppElement::Method(constructor_method));
        
        // StdUnorderedSet class 的析构函数
        stdunorderedset_element.ensure_destructor();
        
        // size 方法
        let size_method = Method {
            method_type: MethodType::Normal,
            name: "size".to_string(),
            return_type: FieldType {
                full_str: "int".to_string(),
                type_str: "int".to_string(),
                type_kind: TypeKind::Int64,
                ptr_level: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        stdunorderedset_element.add_child(HppElement::Method(size_method));
        
        // insert 方法
        let insert_method = Method {
            method_type: MethodType::Normal,
            name: "insert".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "value".to_string(),
                    field_type: (**field_type.value_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdunorderedset_element.add_child(HppElement::Method(insert_method));
        
        // count 方法 (替代 contains，更兼容)
        let count_method = Method {
            method_type: MethodType::Normal,
            name: "count".to_string(),
            return_type: FieldType {
                full_str: "int".to_string(),
                type_str: "int".to_string(),
                type_kind: TypeKind::Int64,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "value".to_string(),
                    field_type: (**field_type.value_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdunorderedset_element.add_child(HppElement::Method(count_method));
        
        // erase 方法
        let erase_method = Method {
            method_type: MethodType::Normal,
            name: "erase".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            params: vec![
                MethodParam {
                    name: "value".to_string(),
                    field_type: (**field_type.value_type.as_ref().unwrap()).clone(),
                },
            ],
            ..Default::default()
        };
        stdunorderedset_element.add_child(HppElement::Method(erase_method));
        
        // clear 方法
        let clear_method = Method {
            method_type: MethodType::Normal,
            name: "clear".to_string(),
            return_type: FieldType {
                full_str: "void".to_string(),
                type_str: "void".to_string(),
                type_kind: TypeKind::Void,
                ptr_level: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        stdunorderedset_element.add_child(HppElement::Method(clear_method));

        return stdunorderedset_element;
    }
}

impl fmt::Debug for HppElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File(arg0) => arg0.fmt(f),
            Self::Class(arg0) => arg0.fmt(f),
            Self::Method(arg0) => arg0.fmt(f),
            Self::Field(arg0) => arg0.fmt(f),
            Self::Enum(arg0) => arg0.fmt(f),
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
            comment_str: field.comment_str.clone(),
            ..Default::default()
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
            comment_str: field.comment_str.clone(),
            ..Default::default()
        };
    }
}

impl FieldType {
    pub fn from_clang_type(clang_type: &Option<clang::Type>) -> Self {
        // println!("clang_type: {:?}, {:?}, {:?}", clang_type, clang_type.unwrap().get_kind(), clang_type.unwrap().get_template_argument_types());

        let mut display_name = clang_type.unwrap().get_display_name();
        let mut field_type = FieldType::default();
        field_type.full_str = display_name.clone();
        // 去掉修饰符
        display_name = display_name.replace("const ", "");

        let mut lower_full_str = display_name.to_lowercase();
        // enum - 检测是否为枚举类型，但保留类型名称用于后续处理
        let mut is_enum_type = false;
        if let Some(elaborated) = clang_type.unwrap().get_elaborated_type() {
            if elaborated.get_kind() == clang::TypeKind::Enum {
                is_enum_type = true;
                // 不再强制转换为 "int"，保留枚举类型名称
            }
        }

        // 一些特殊处理的类型
        // std::string - 需要处理引用类型
        let clean_string_type = lower_full_str
            .replace("&", "")
            .replace("*", "")
            .trim()
            .to_string();
        
        if clean_string_type == "std::string" || clean_string_type == "string" {
            field_type.type_kind = TypeKind::String;
            field_type.full_str = "std::string".to_string();
            field_type.type_str = "String".to_string();
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
        // 数组
        else if clang_type.unwrap().get_kind() == clang::TypeKind::ConstantArray {
            lower_full_str = lower_full_str.split_once("[").unwrap_or((&lower_full_str, "")).0.trim().to_string();
            field_type.ptr_level = 1;
        }
        // std::vector
        else if lower_full_str.starts_with("std::vector") {
            field_type.type_kind = TypeKind::StdVector;
            field_type.type_str = display_name.clone();

            let template_args = clang_type.unwrap().get_template_argument_types().unwrap_or_default();
            let value_clang_type = template_args.first().unwrap();
            let value_type = FieldType::from_clang_type(value_clang_type);

            field_type.value_type = Some(Box::new(value_type));
            return field_type;
        }
        // std::map
        else if lower_full_str.starts_with("std::map") {
            field_type.type_kind = TypeKind::StdMap;
            field_type.type_str = display_name.clone();

            let template_args = clang_type.unwrap().get_template_argument_types().unwrap_or_default();
            if template_args.len() >= 2 {
                let key_clang_type = template_args.get(0).unwrap();
                let value_clang_type = template_args.get(1).unwrap();
                
                let key_type = FieldType::from_clang_type(key_clang_type);
                let value_type = FieldType::from_clang_type(value_clang_type);

                field_type.key_type = Some(Box::new(key_type));
                field_type.value_type = Some(Box::new(value_type));
            }
            return field_type;
        }
        // std::unordered_map
        else if lower_full_str.starts_with("std::unordered_map") {
            field_type.type_kind = TypeKind::StdUnorderedMap;
            field_type.type_str = display_name.clone();

            let template_args = clang_type.unwrap().get_template_argument_types().unwrap_or_default();
            if template_args.len() >= 2 {
                let key_clang_type = template_args.get(0).unwrap();
                let value_clang_type = template_args.get(1).unwrap();
                
                let key_type = FieldType::from_clang_type(key_clang_type);
                let value_type = FieldType::from_clang_type(value_clang_type);

                field_type.key_type = Some(Box::new(key_type));
                field_type.value_type = Some(Box::new(value_type));
            }
            return field_type;
        }
        // std::set
        else if lower_full_str.starts_with("std::set") {
            field_type.type_kind = TypeKind::StdSet;
            field_type.type_str = display_name.clone();

            let template_args = clang_type.unwrap().get_template_argument_types().unwrap_or_default();
            let value_clang_type = template_args.first().unwrap();
            let value_type = FieldType::from_clang_type(value_clang_type);

            field_type.value_type = Some(Box::new(value_type));
            return field_type;
        }
        // std::unordered_set
        else if lower_full_str.starts_with("std::unordered_set") {
            field_type.type_kind = TypeKind::StdUnorderedSet;
            field_type.type_str = display_name.clone();

            let template_args = clang_type.unwrap().get_template_argument_types().unwrap_or_default();
            let value_clang_type = template_args.first().unwrap();
            let value_type = FieldType::from_clang_type(value_clang_type);

            field_type.value_type = Some(Box::new(value_type));
            return field_type;
        }

        // 计算指针级别
        if field_type.ptr_level == 0 {
            let ptr_level = lower_full_str.chars().rev().take_while(|&c| c == '*').count();
            field_type.ptr_level = ptr_level as i32;
        }

        // 如果是枚举类型，特殊处理
        if is_enum_type {
            field_type.type_kind = TypeKind::Enum;
            field_type.type_str = display_name.clone();
            return field_type;
        }

        let lower_full_str_without_ptr = lower_full_str.trim_end_matches('*').trim();
        match lower_full_str_without_ptr {
            "void" => {
                field_type.type_kind = TypeKind::Void;
                field_type.type_str = "void".to_string();
            }
            "int" | "int64_t" | "size_t" | "uint64_t" => {
                field_type.type_kind = TypeKind::Int64;
                match lower_full_str_without_ptr {                
                    "int64_t" | "size_t" | "uint64_t" => {
                        field_type.type_str = "int64_t".to_string();
                    }
                    "int" => {
                        field_type.type_str = "int".to_string();
                    }
                    _ => {
                        field_type.type_str = "int".to_string();
                    }
                }
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
            "unsigned char" => {
                field_type.type_kind = TypeKind::Char;
                field_type.type_str = "unsigned char".to_string();
            }
            "bool" => {
                field_type.type_kind = TypeKind::Bool;
                field_type.type_str = "bool".to_string();
            }
            "std::uint8_t" | "uint8_t" | "std::uint16_t" | "uint16_t" |
            "std::uint32_t" | "uint32_t" | "std::int8_t" | "int8_t" |
            "std::int16_t" | "int16_t" | "std::int32_t" | "int32_t" => {
                // std 整数类型映射到 Int64
                field_type.type_kind = TypeKind::Int64;
                field_type.type_str = "int64_t".to_string();
            }
            _ => {
                // 获取类型名称用于检查
                let type_display_name = if let Some(_pointee) = clang_type.unwrap().get_pointee_type() {
                    clang_type.unwrap().get_pointee_type().unwrap().get_display_name()
                } else {
                    clang_type.unwrap().get_display_name()
                };

                // 检查是否应该被忽略
                if should_ignore_type(&type_display_name) {
                    field_type.type_kind = TypeKind::Ignored;
                    field_type.type_str = type_display_name;
                } else {
                    // 指针类型
                    if let Some(_pointee) = clang_type.unwrap().get_pointee_type() {
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

        }

        return field_type;
    }

    pub fn new_void() -> Self {
        return FieldType {
            full_str: "void".to_string(),
            type_str: "void".to_string(),
            ..Default::default()
        };
    }

    pub fn get_value_type_str(&self) -> String {
        if self.value_type.is_none() {
            return "".to_string();
        }
        let value_type = self.value_type.as_ref().unwrap();
        if value_type.type_kind == TypeKind::StdPtr {
            return format!("StdPtr_{}", value_type.type_str);
        } else {
            return value_type.type_str.clone();
        }
    }

    pub fn get_key_type_str(&self) -> String {
        if self.key_type.is_none() {
            return "".to_string();
        }
        let key_type = self.key_type.as_ref().unwrap();
        if key_type.type_kind == TypeKind::StdPtr {
            return format!("StdPtr_{}", key_type.type_str);
        } else {
            return key_type.type_str.clone();
        }
    }
}
