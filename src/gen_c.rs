use std::{fs, io::Write, path::{Path, PathBuf}};

use crate::gen_context::*;

pub fn gen_c(gen_context: &GenContext, gen_out_dir: &str) {
    match gen_context.hpp_elements.first().unwrap() {
        HppElement::File(file) => {
            gen_c_file(gen_context, file, gen_out_dir);
        }
        _ => {
            unimplemented!("gen_c: first element is not File");
        }
    }
}

#[derive(Debug)]
struct CFileContext<'a> {
    pub ch_str: &'a mut String,
    pub cc_str: &'a mut String,

    pub gen_context: &'a GenContext,
}

fn gen_c_file(gen_context: &GenContext, file: &File, gen_out_dir: &str) {
    let hpp_filename = Path::new(&file.path).file_name().unwrap().to_os_string().into_string().unwrap();
    let filename_without_ext = match hpp_filename.rfind(".") {
        Some(idx) => &hpp_filename[..idx],
        None => &hpp_filename,
    };
    let h_filename = format!("{}_ffi.h", filename_without_ext);
    let ch_path = PathBuf::new().join(gen_out_dir).join(h_filename.clone()).into_os_string().into_string().unwrap();
    let mut ch_file = fs::File::create(ch_path).unwrap();

    let c_filename = format!("{}_ffi.cpp", filename_without_ext);
    let cc_path = PathBuf::new().join(gen_out_dir).join(c_filename.clone()).into_os_string().into_string().unwrap();
    let mut cc_file = fs::File::create(cc_path).unwrap();

    let mut ch_str = String::new();
    // 公共头
    let mut ch_header = format!("
#include <stdio.h>

#define API_EXPORT __attribute__((visibility(\"default\"))) __attribute__((used))

extern \"C\" {{
");
    // 收集所有需要生成 typedef 的类型名
    let mut typedef_names = vec![];
    // 收集所有被引用的 StdPtr 类型
    let mut stdptr_types = vec![];

    // 1. 首先收集文件中定义的类
    for element in &file.children {
        match element {
            HppElement::Class(class) => {
                let typedef_name = class.type_str.to_string();
                if !typedef_names.contains(&typedef_name) {
                    typedef_names.push(typedef_name.to_string());
                }
            }
            _ => {}
        }
    }

    // 2. 然后收集所有方法中引用的类型
    collect_referenced_types(file, &mut typedef_names, &mut stdptr_types);

    // 3. 为所有收集到的类型生成 typedef
    for typedef_name in &typedef_names {
        ch_header.push_str(&format!("typedef void* FFI_{};\n", typedef_name));
    }

    // 4. 为所有收集到的 StdPtr 类型生成 typedef
    for stdptr_type in &stdptr_types {
        ch_header.push_str(&format!("typedef void* FFI_StdPtr_{};\n", stdptr_type));
    }

    ch_str.push_str(&ch_header);
    let mut cc_str = String::new();
    let cc_header = format!("
#include \"{}\"
#include \"{}\"
#include \"dart_api_dl.h\"

extern \"C\" {{

", h_filename, hpp_filename);
    cc_str.push_str(&cc_header);

    let mut c_context = CFileContext{
        ch_str: &mut ch_str,
        cc_str: &mut cc_str,
        gen_context: &gen_context,
    };

    for child in &file.children {
        match child {
            HppElement::Class(class) => {
                if class.is_callback() {
                    gen_c_callback_class(&mut c_context, class);
                } else {
                    gen_c_class(&mut c_context, class);
                }
            }
            // 独立函数
            HppElement::Method(method) => {
                gen_c_class_method(&mut c_context, None, method);
            }
            _ => {
                unimplemented!("gen_c_file: unknown child, {:?}", child);
            }
        }
    }

    // 公共尾
    let ch_footer = r#"
} // extern "C"
"#;
    ch_str.push_str(&ch_footer);
    let cc_footer = r#"
} // extern "C"
"#;
    cc_str.push_str(&cc_footer);

    ch_file.write_all(ch_str.as_bytes());
    cc_file.write_all(cc_str.as_bytes());
}

fn gen_c_class(c_context: &mut CFileContext, class: &Class) {
    let c_class_decl = format!("\n");
c_context.ch_str.push_str(&c_class_decl);

    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                gen_c_class_method(c_context, Some(&class), method);
            }
            HppElement::Field(field) => {
                let (get_decl, set_decl) = get_str_field_decl(Some(&class), field);
                c_context.ch_str.push_str(&format!("{}\n", get_decl));
                c_context.ch_str.push_str(&format!("{}\n", set_decl));

                let (get_impl, set_impl) = get_str_field_impl(Some(&class), field);
                c_context.cc_str.push_str(&format!("{}\n", get_impl));
                c_context.cc_str.push_str(&format!("{}\n", set_impl));
            }
            _ => {
                unimplemented!("gen_c_class: unknown child, {:?}", child);
            }
        }
    }
}

fn gen_c_class_method(c_context: &mut CFileContext, class: Option<&Class>, method: &Method) {
    let method_decl = get_str_method_decl(class, method);
    let method_impl = get_str_method_impl(class, method);

    c_context.ch_str.push_str(format!("{}\n", method_decl).as_str());
    c_context.cc_str.push_str(format!("{}\n", method_impl).as_str());
}

/// 回调类
fn gen_c_callback_class(c_context: &mut CFileContext, class: &Class) {
    let c_class_callback_decl = format!("\n");
    c_context.ch_str.push_str(&c_class_callback_decl);

    // 子类化的回调类的名字
    let subclass_name = format!("Impl_{}", class.type_str);

    // 注册函数的声明和实现
    let mut regist_decl = String::new();
    let mut regist_var_decl = String::new();
    let mut regist_impl = String::new();
    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                // 这次只生成需要重写的回调函数
                if method.method_type == MethodType::Normal {
                    let (local_regist_decl, local_regist_var_decl, local_regist_impl) = get_str_callback_method_regist(Some(&class), method);
                    regist_decl.push_str(&local_regist_decl);
                    regist_var_decl.push_str(&local_regist_var_decl);
                    regist_impl.push_str(&local_regist_impl);
                }
            }
            HppElement::Field(field) => {
                // TODO
            }
            _ => {
                unimplemented!("gen_c_callback_class: unknown child");
            }
        }
    }

    // 生成回调子类
    let mut c_class_callback_impl = format!("{}
class {} : {} {{
public:
", regist_var_decl, subclass_name, class.type_str);
c_context.cc_str.push_str(&c_class_callback_impl);
    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                // 这次只生成需要重写的回调函数
                if method.method_type == MethodType::Normal {
                    let callback_method_impl = get_str_callback_method_impl(Some(&class), method);
                    c_context.cc_str.push_str(&callback_method_impl);
                }
            }
            HppElement::Field(field) => {
                // TODO
            }
            _ => {
                unimplemented!("gen_c_callback_class: unknown child");
            }
        }
    }
    c_context.cc_str.push_str("\n};\n");

    // 生成回调子类的其他正常函数
    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                gen_c_class_method(c_context, Some(&class), method);
                // 作为回调的抽象类并不能new，所以这里换成可实例化的子类
                let form_new = format!("new {}", class.type_str);
                let to_new = format!("new {}", subclass_name);
                if let Some(pos) = c_context.cc_str.find(&form_new) {
                    let end = pos + form_new.len();
                    c_context.cc_str.replace_range(pos..end, &to_new);
                }
            }
            HppElement::Field(field) => {
                // TODO
            }
            _ => {
                unimplemented!("gen_c_callback_class: unknown child");
            }
        }
    }

    // 生成注册函数的实现
    c_context.cc_str.push_str(&regist_impl);

    c_context.cc_str.push_str("\n");

    // 写.h
    c_context.ch_str.push_str(&format!("{}", regist_decl));
}

/// (get, set)
fn get_str_field_decl(class: Option<&Class>, field: &Field) -> (String, String) {
    // ffi 中的类型名
    let ffi_class_name = format!("FFI_{}", class.unwrap().type_str);
    let cur_class_name = class.get_class_name_or_empty();

    let get_decl = format!("API_EXPORT {} ffi_{}_get_{}({} obj);", 
        get_str_ffi_type(&field.field_type), cur_class_name, field.name, ffi_class_name);
    let set_decl = format!("API_EXPORT void ffi_{}_set_{}({} obj, {} {});", 
        cur_class_name, field.name, ffi_class_name, get_str_ffi_type(&field.field_type), field.name);

    return (get_decl, set_decl);
}

/// (get, set)
fn get_str_field_impl(class: Option<&Class>, field: &Field) -> (String, String) {
    let cur_class_name = class.get_class_name_or_empty();

    let (local_get_decl, local_set_decl) = get_str_field_decl(class, field); 
    let get_decl = local_get_decl.trim_end_matches(";");
    let set_decl = local_set_decl.trim_end_matches(";");

    let get_impl_body = get_str_method_impl_body(class, &field.field_type, &field.name, None);
    let get_impl = format!("{} {{
    {}* ptr = ({}*)obj;
    {}
}}",
        get_decl,
        cur_class_name, cur_class_name,
        get_impl_body,
    );
    let ffi_to_cpp_param = get_str_ffi_to_cpp_param_field(&field.field_type, &field.name);
    let mut set_impl_body = format!("ptr->{} = {};", field.name, ffi_to_cpp_param);
    if field.field_type.ptr_level > 0 && field.field_type.full_str.contains("[") {
        set_impl_body = format!("memcpy(ptr->{}, {}, sizeof(ptr->{}));", field.name, ffi_to_cpp_param, field.name);        
    }
    let set_impl = format!("{} {{
    {}* ptr = ({}*)obj;
    {}
}}",
        set_decl,
        cur_class_name, cur_class_name,
        set_impl_body,
    );

    return (get_impl, set_impl);
}

// /// 回调方法的实现
// fn get_str_callback_method_impl(class: Option<&Class>, method: &Method) -> String {
//     if (method.method_type == MethodType::Constructor) || (method.method_type == MethodType::Destructor) {
//         return "".to_string();
//     }

//     // ffi 中的类型名
//     let ffi_class_name = format!("FFI_{}", class.unwrap().type_str);
//     // 指向函数指针的变量
//     let fun_ptr_var_str = format!("{}_{}", class.unwrap().type_str, method.name);

//     // .cpp 中的实现
//     // 调用函数指针的函数实现
//     let mut c_class_callback_method_impl = format!("    virtual {} {}(", 
//         method.return_type.full_str, method.name);
//     for param in &method.params {
//         c_class_callback_method_impl.push_str(&format!("{} {}, ", param.field_type.full_str, param.name));
//     }
//     if !method.params.is_empty() {
//         c_class_callback_method_impl.truncate(c_class_callback_method_impl.len() - ", ".len()); // 去掉最后一个参数的, 
//     }
//     c_class_callback_method_impl.push_str(&format!(") override {{
//         return {}(({})this, ", 
//         fun_ptr_var_str, ffi_class_name));
//     for param in &method.params {
//         c_class_callback_method_impl.push_str(&format!("({}){}, ", get_str_ffi_type(&param.field_type), param.name));
//     }
//     if !method.params.is_empty() {
//         c_class_callback_method_impl.truncate(c_class_callback_method_impl.len() - ", ".len()); // 去掉最后一个参数的, 
//     }
//     c_class_callback_method_impl.push_str(");\n\t};\n");

//     return c_class_callback_method_impl;
// }

/// 回调方法的实现
fn get_str_callback_method_impl(class: Option<&Class>, method: &Method) -> String {
    if (method.method_type == MethodType::Constructor) || (method.method_type == MethodType::Destructor) {
        return "".to_string();
    }

    // ffi 中的类型名
    let ffi_class_name = format!("FFI_{}", class.unwrap().type_str);
    // 指向函数指针的变量
    let fun_ptr_var_str = format!("{}_{}", class.unwrap().type_str, method.name);

    // .cpp 中的实现
    // 调用函数指针的函数实现
    let args_num = method.params.len()+1;

    let mut decl_params = Vec::new();
    for param in &method.params {
        decl_params.push(format!("{} {}", param.field_type.full_str, param.name));
    }
    let decl_params_str = decl_params.join(", ");

    let mut gen_values = Vec::new();
    let mut values = vec!["&value0".to_string()];
    for i in 0..method.params.len() {
        let param = method.params.get(i).unwrap();
        let (dart_type_enum, dart_type_set_value, convert_str) = get_str_callback_method_impl_dart_cobject_type(&param.field_type);
        let mut param_name = param.name.clone();
        if (param.field_type.type_kind == TypeKind::String) {
            param_name = format!("{}.c_str()", param_name);
        }
        else if (param.field_type.type_kind == TypeKind::Class) && (param.field_type.ptr_level == 0) {
            param_name = format!("(new {}({}))", param.field_type.type_str, param_name);
        }
        else if (param.field_type.type_kind == TypeKind::StdPtr || param.field_type.type_kind == TypeKind::StdVector) {
            param_name = format!("(new {}({}))", param.field_type.full_str, param_name);
        }
        gen_values.push(format!("
        Dart_CObject value{};
        value{}.type = {};
        value{}.value.{} = ({}){};
        ",
        i+1,
        i+1, dart_type_enum,
        i+1, dart_type_set_value, convert_str, param_name,
        ));

        values.push(format!("&value{}", i+1));
    }
    let gen_values_str = gen_values.join("");
    let values_str = values.join(", ");

    let ret_str = format!("    virtual {} {}({}) override {{
        {}

        Dart_CObject value0;
        value0.type = Dart_CObject_kInt64;
        value0.value.as_int64 = (int64_t)this;

        Dart_CObject* values[] = {{{}}};
        Dart_CObject args;
        args.type = Dart_CObject_kArray;
        args.value.as_array.length = {};
        args.value.as_array.values = values;
        Dart_PostCObject_DL((Dart_Port_DL){}, &args);
}};
",
        method.return_type.full_str, method.name, decl_params_str,
        gen_values_str,
        values_str,
        args_num,
        fun_ptr_var_str,
    );

    return ret_str;
}

/**
 * Dart_CObject 枚举类型, value.xxx 类型, 从C到Dart类型转换类型
 * 需要特殊处理的类型，会返回空字符串
 */
fn get_str_callback_method_impl_dart_cobject_type(field_type: &FieldType) -> (String, String, String) {
    match field_type.type_kind {
        TypeKind::Void => {
            return ("Dart_CObject_kNull".to_string(), "as_int64".to_string(), "int64_t".to_string());
        }
        TypeKind::Int64 => {
            return ("Dart_CObject_kInt64".to_string(), "as_int64".to_string(), "int64_t".to_string());
        }
        TypeKind::Float | TypeKind::Double => {
            return ("Dart_CObject_kDouble".to_string(), "as_double".to_string(), "double".to_string());
        }
        TypeKind::Char => {
            return ("Dart_CObject_kString".to_string(), "as_string".to_string(), "char*".to_string());
        }
        TypeKind::Bool => {
            return ("Dart_CObject_kBool".to_string(), "as_bool".to_string(), "bool".to_string());
        }
        TypeKind::String => {
            return ("Dart_CObject_kString".to_string(), "as_string".to_string(), "char*".to_string());
        }
        TypeKind::Class | TypeKind::StdPtr | TypeKind::StdVector => {
            return ("Dart_CObject_kInt64".to_string(), "as_int64".to_string(), "int64_t".to_string());
        }
        _ => {
            return ("".to_string(), "".to_string(), "".to_string());
        }
    }
}

// /// 生成注册函数的定义
// /// (.h中的函数指针类型和注册函数定义，.cpp中的函数指针变量定义，.cpp中的注册函数实现)
// fn get_str_callback_method_regist(class: Option<&Class>, method: &Method) -> (String, String, String) {
//     if (method.method_type == MethodType::Constructor) || (method.method_type == MethodType::Destructor) {
//         return ("".to_string(), "".to_string(), "".to_string());
//     }

//     // ffi 中的类型名
//     let ffi_class_name = format!("FFI_{}", class.unwrap().type_str);
//     // 函数指针类型的名字
//     let fun_ptr_type_str = format!("{}_{}", ffi_class_name, method.name);
//     // 指向函数指针的变量
//     let fun_ptr_var_str = format!("{}_{}", class.unwrap().type_str, method.name);
//     // 函数参数定义列表
//     let params_decl_str = get_str_params_decl(class, method);

//     // .h中的函数指针类型和注册函数定义
//     // 1. 函数指针类型声明
//     // 2. 注册函数指针的函数声明
//     let regist_decl = format!("typedef {} (*{})({});
// void {}_regist({} {});
// ",
//         get_str_ffi_type(&method.return_type), fun_ptr_type_str, params_decl_str,
//         fun_ptr_type_str, fun_ptr_type_str, method.name
//     );

//     // .cpp中的函数指针变量定义
//     // 1. 注册函数指针的实现
//     let regist_var_decl = format!("static {} {} = nullptr;\n", fun_ptr_type_str, fun_ptr_var_str);

//     // .cpp中的注册函数实现
//     let regist_impl = format!("void {}_regist({} {}){{
//     {} = {};
// }};
// ", fun_ptr_type_str, fun_ptr_type_str, method.name, fun_ptr_var_str, method.name);

//     return (regist_decl, regist_var_decl, regist_impl);
// }

/// 生成注册函数的定义
/// (.h中的函数指针类型和注册函数定义，.cpp中的函数指针变量定义，.cpp中的注册函数实现)
fn get_str_callback_method_regist(class: Option<&Class>, method: &Method) -> (String, String, String) {
    if (method.method_type == MethodType::Constructor) || (method.method_type == MethodType::Destructor) {
        return ("".to_string(), "".to_string(), "".to_string());
    }

    // ffi 中的类型名
    let ffi_class_name = format!("FFI_{}", class.unwrap().type_str);
    // 函数指针类型的名字
    let fun_ptr_type_str = format!("{}_{}", ffi_class_name, method.name);
    // 指向函数指针的变量
    let fun_ptr_var_str = format!("{}_{}", class.unwrap().type_str, method.name);
    // 函数参数定义列表
    let params_decl_str = get_str_params_decl(class, method);

    // .h中的函数指针类型和注册函数定义
    // 1. 函数指针类型声明
    // 2. 注册函数指针的函数声明
    let regist_decl = format!("typedef {} (*{})({});
API_EXPORT void {}_regist(int64_t {});
",
        get_str_ffi_type(&method.return_type), fun_ptr_type_str, params_decl_str,
        fun_ptr_type_str, method.name
    );

    // .cpp中的函数指针变量定义
    // 1. 注册函数指针的实现
    let regist_var_decl = format!("static int64_t {} = 0;\n", fun_ptr_var_str);

    // .cpp中的注册函数实现
    let regist_impl = format!("API_EXPORT void {}_regist(int64_t {}){{
    {} = {};
}};
", 
    fun_ptr_type_str, method.name, 
    fun_ptr_var_str, method.name,
);

    return (regist_decl, regist_var_decl, regist_impl);
}

fn get_str_ffi_type(field_type: &FieldType) -> String {
    match field_type.type_kind {
        TypeKind::Void | TypeKind::Int64 | TypeKind::Float | TypeKind::Double | TypeKind::Char | TypeKind::Bool => {
            if field_type.ptr_level == 0 {
                return field_type.type_str.clone();
            } else {
                return format!("{}{}", field_type.type_str, "*".repeat(field_type.ptr_level as usize));
            }
        }
        TypeKind::String => {
            return "const char*".to_string();
        }
        TypeKind::Class => {
            return format!("FFI_{}", field_type.type_str);
        }
        TypeKind::StdPtr => {
            return format!("FFI_StdPtr_{}", field_type.type_str);
        }
        TypeKind::StdVector => {
            let value_type = field_type.value_type.as_deref().unwrap();
            if (value_type.type_kind == TypeKind::String) {
                return format!("FFI_StdVector_String");
            } else {
                return format!("FFI_StdVector_{}", field_type.get_value_type_str());
            }
        }
        _ => {
            unimplemented!("get_ffi_type_str: unknown type kind");
        }
    }
}

fn get_str_method_decl(class: Option<&Class>, method: &Method) -> String {
    let ffi_decl_name = get_str_ffi_decl_class_name(class, method);
    let params = get_str_params_decl(class, method);
    let method_decl = format!("API_EXPORT {} {}({});", 
        get_str_ffi_type(&method.return_type), ffi_decl_name, params);

    return method_decl;
}

fn get_str_method_impl(class: Option<&Class>, method: &Method) -> String {
    let decl_class_name = get_str_decl_class_name(class, method);
    let method_decl = get_str_method_decl(class, method);
    // 去掉函数定义的最后一个分号，作为函数实现的第一行
    let method_prefix = method_decl.trim_end_matches(";");
    let (param_prefix, param_str) = get_str_params_impl(class, method);
    let impl_return_type = get_str_ffi_type(&method.return_type);

    let mut method_impl = String::new();
    match method.method_type {
        MethodType::Constructor => {
            if method.return_type.type_kind == TypeKind::StdPtr {
                method_impl = format!("{} {{
    {}
    return ({})new std::shared_ptr<{}>({});
}};", method_prefix, param_prefix, impl_return_type, method.return_type.type_str, param_str);
            }
            else if method.return_type.type_kind == TypeKind::StdVector {
                method_impl = format!("{} {{
    {}
    return ({})new std::shared_ptr<{}>({});
}};", method_prefix, param_prefix, impl_return_type, method.return_type.type_str, param_str);
            } 
            else {
                method_impl = format!("{} {{
    {}
    return ({})new {}({});
}};", method_prefix, param_prefix, impl_return_type, decl_class_name, param_str);
            }
        }
        MethodType::Destructor => {
            method_impl = format!("{} {{
    {}
    if (ptr != nullptr) {{
        return delete ptr;
    }}
}};", method_prefix, param_prefix)
        }
        MethodType::Normal => {
            let impl_body = get_str_method_impl_body(class, &method.return_type, &method.name, Some(&param_str));
            method_impl = format!("{} {{
    {}
    {}
}};", method_prefix, param_prefix, impl_body);
        }
        _ => {
            unimplemented!("gen_c_class_method_impl: unknown method type");
        }
    }

    return method_impl
}

fn get_str_method_impl_body(class: Option<&Class>, return_field_type: &FieldType, method_name: &str, param_str: Option<&str>) -> String {
    let impl_return_type = get_str_ffi_type(&return_field_type);
    
    // 对于静态方法，调用使用类名作为前缀，例如 ClassName::staticMethod()
    // 对于普通方法，使用 ptr-> 前缀
    let is_static = if let Some(cls) = class {
        match class {
            Some(c) => match c.children.iter().find(|e| if let HppElement::Method(m) = e { m.name == method_name } else { false }) {
                Some(HppElement::Method(m)) => m.is_static,
                _ => false
            },
            None => false,
        }
    } else {
        false
    };

    let call_prefix = if class.get_class_name_or_empty().is_empty() { 
        "" 
    } else if is_static { 
        &format!("{}::", class.get_class_name_or_empty()) 
    } else { 
        "ptr->" 
    };
    
    // 带有括号的参数列表，如果没有参数则为空字符串（有些直接访问变量的操作，不需要括号）
    let full_param_str = if param_str.is_none() { 
        "" 
    } else { 
        &format!("({})", param_str.unwrap()) 
    };
    
    if return_field_type.type_kind == TypeKind::String {
        return format!("static std::string retStr = \"\";
    retStr = {}{}{};
    return (const char*)retStr.c_str();", call_prefix, method_name, full_param_str);
    } 
    else if (return_field_type.type_kind == TypeKind::Class && 0 == return_field_type.ptr_level) {
        return format!("return ({})new {}({}{}{});", impl_return_type, return_field_type.type_str, call_prefix, method_name, full_param_str);
    }
    else if (return_field_type.type_kind == TypeKind::StdPtr && 0 == return_field_type.ptr_level) 
    || (return_field_type.type_kind == TypeKind::StdVector && 0 == return_field_type.ptr_level) 
    {
        return format!("return ({})new {}({}{}{});", impl_return_type, return_field_type.full_str, call_prefix, method_name, full_param_str);
    }
    else {
        return format!("return ({}){}{}{};", impl_return_type, call_prefix, method_name, full_param_str);
    }
}

/// 函数是不是需要加第一个类的实例参数，模拟调用类实例的调用方法
pub fn get_is_need_first_class_param(class: Option<&Class>, method: &Method) -> bool {
    if method.is_static {
        return false;
    }
    match method.method_type {
        MethodType::Constructor => {
            return false;
        }
        MethodType::Destructor => {
            return true;
        }
        MethodType::Normal => {
            if class.is_some() {
                return true;
            }
            return false;
        }
        _ => {
            unimplemented!("method_get_is_need_first_class_param: unknown method type");
        }
    }
}

/// 返回函数的类名前缀或者空字符串
fn get_str_decl_class_name<'a>(class: Option<&'a Class>, method: &Method) -> &'a str {
    if let Some(cur_class) = class {
        return &cur_class.type_str;
    }
    return "";
}

/// 返回函数的 ffi 声明名
fn get_str_ffi_decl_class_name(class: Option<&Class>, method: &Method) -> String {
    let mut cur_class_name = "";
    if let Some(cur_class) = class {
        cur_class_name = &cur_class.type_str;
    }

    return format!("ffi_{}_{}", cur_class_name, method.name);
}

/// 返回声明参数列表字符串
fn get_str_params_decl(class: Option<&Class>, method: &Method) -> String {
    let mut param_strs = Vec::new();
    if get_is_need_first_class_param(class, method) {
        param_strs.push(format!("FFI_{} obj", get_str_decl_class_name(class, method)));
    }
    for param in &method.params {
        param_strs.push(format!("{} {}", get_str_ffi_type(&param.field_type), param.name));
    }

    return param_strs.join(", ");
}

/// 返回实现中的调用参数列表字符串，(前置条件, 调用参数)
fn get_str_params_impl(class: Option<&Class>, method: &Method) -> (String, String) {
    let mut param_prefixs = Vec::new();
    let mut param_strs = Vec::new();
    if get_is_need_first_class_param(class, method) {
        if let Some(cur_class) = class {
            if cur_class.class_type == ClassType::StdPtr {
                let suffix = cur_class.type_str.split_once("_").unwrap_or(("", "")).1;
                param_prefixs.push(format!("std::shared_ptr<{}>* ptr = (std::shared_ptr<{}>*)obj;", suffix, suffix));
            }
            else if cur_class.class_type ==  ClassType::StdVector {
                let suffix = cur_class.value_type.as_deref().unwrap().full_str.clone();
                param_prefixs.push(format!("std::vector<{}>* ptr = (std::vector<{}>*)obj;", suffix, suffix));
            }
            else {
                param_prefixs.push(format!("{}* ptr = ({}*)obj;", cur_class.type_str, cur_class.type_str));
            }
        } else {
            unimplemented!("method_build_params_impl: need first class param but class is None");
        }
    }

    for param in &method.params {
        param_strs.push(get_str_ffi_to_cpp_param_field(&param.field_type, &param.name));
    }

    let param_prefixs_str = param_prefixs.join("\n");
    let param_strs_str = param_strs.join(", ");
    return (param_prefixs_str, param_strs_str);
}

fn get_str_ffi_to_cpp_param_field(field_type: &FieldType, param_name: &str) -> String {
    if field_type.type_kind == TypeKind::String {
        return format!("std::string({})", param_name);
    }
    else if (field_type.type_kind == TypeKind::Class && 0 == field_type.ptr_level) {
        return format!("({})(*({}*){})", &field_type.full_str, field_type.type_str, param_name);
    }
    else if (field_type.type_kind == TypeKind::StdPtr && 0 == field_type.ptr_level)
    || (field_type.type_kind == TypeKind::StdVector && 0 == field_type.ptr_level)
    {
        return format!("({})(*({}*){})", &field_type.full_str, &field_type.full_str, param_name);
    } 
    else {
        if field_type.ptr_level > 0 {
            return format!("({}{}){}", &field_type.type_str, "*".repeat(field_type.ptr_level as usize), param_name);
        } else {
            return format!("({}){}", &field_type.full_str, param_name);
        }
    }
}

/// 收集所有在方法参数和返回值中被引用的类型
fn collect_referenced_types(file: &File, typedef_names: &mut Vec<String>, stdptr_types: &mut Vec<String>) {
    // 递归收集文件中所有元素引用的类型
    collect_element_referenced_types(file.children.first().unwrap(), typedef_names, stdptr_types);
}

/// 递归处理HppElement，收集其中引用的所有类型
fn collect_element_referenced_types(element: &HppElement, typedef_names: &mut Vec<String>, stdptr_types: &mut Vec<String>) {
    match element {
        HppElement::File(file) => {
            for child in &file.children {
                collect_element_referenced_types(child, typedef_names, stdptr_types);
            }
        },
        HppElement::Class(class) => {
            // 收集类中所有子元素引用的类型
            for child in &class.children {
                collect_element_referenced_types(child, typedef_names, stdptr_types);
            }
        },
        HppElement::Method(method) => {
            // 处理返回类型
            collect_field_type(&method.return_type, typedef_names, stdptr_types);
            
            // 处理参数类型
            for param in &method.params {
                collect_field_type(&param.field_type, typedef_names, stdptr_types);
            }
        },
        HppElement::Field(field) => {
            // 处理字段类型
            collect_field_type(&field.field_type, typedef_names, stdptr_types);
        }
    }
}

/// 处理单个字段类型，收集需要的typedef
fn collect_field_type(field_type: &FieldType, typedef_names: &mut Vec<String>, stdptr_types: &mut Vec<String>) {
    match field_type.type_kind {
        TypeKind::Class => {
            // 添加类类型
            if !typedef_names.contains(&field_type.type_str) {
                typedef_names.push(field_type.type_str.clone());
            }
        },
        TypeKind::StdPtr => {
            // 添加StdPtr的基类类型
            if !typedef_names.contains(&field_type.type_str) {
                typedef_names.push(field_type.type_str.clone());
            }
            // 添加StdPtr类型本身
            if !stdptr_types.contains(&field_type.type_str) {
                stdptr_types.push(field_type.type_str.clone());
            }
        },
        TypeKind::StdVector => {
            // 处理vector内部的值类型
            if let Some(value_type) = &field_type.value_type {
                collect_field_type(value_type, typedef_names, stdptr_types);
                
                // 添加StdVector类型本身
                let vector_type_str = format!("StdVector_{}", value_type.get_value_type_str());
                if !typedef_names.contains(&vector_type_str) {
                    typedef_names.push(vector_type_str);
                }
            }
        },
        _ => {} // 其他基本类型不需要特殊处理
    }
}
