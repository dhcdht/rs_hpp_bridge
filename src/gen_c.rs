use std::{fs, io::Write, path::{Path, PathBuf}, str::FromStr};

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
    let h_filename = hpp_filename.replace(".hpp", "_ffi.h");
    let ch_path = PathBuf::new().join(gen_out_dir).join(h_filename.clone()).into_os_string().into_string().unwrap();
    let mut ch_file = fs::File::create(ch_path).unwrap();

    let c_filename = h_filename.replace(".h", ".cpp");
    let cc_path = PathBuf::new().join(gen_out_dir).join(c_filename.clone()).into_os_string().into_string().unwrap();
    let mut cc_file = fs::File::create(cc_path).unwrap();

    let mut ch_str = String::new();
    // 公共头
    let mut ch_header = format!("
#include <stdio.h>

extern \"C\" {{
");
    for class_name in &gen_context.class_names {
        ch_header.push_str(&format!("typedef long FFI_{};\n", class_name));
    }
    ch_str.push_str(&ch_header);
    let mut cc_str = String::new();
    let cc_header = format!("
#include \"{}\"
#include \"{}\"

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
                // TODO
            }
            _ => {
                unimplemented!("gen_c_class: unknown child");
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
    let mut c_class_callback_method_impl = format!("    virtual {} {}(", 
        method.return_type.full_str, method.name);
    for param in &method.params {
        c_class_callback_method_impl.push_str(&format!("{} {}, ", param.field_type.full_str, param.name));
    }
    if !method.params.is_empty() {
        c_class_callback_method_impl.truncate(c_class_callback_method_impl.len() - ", ".len()); // 去掉最后一个参数的, 
    }
    c_class_callback_method_impl.push_str(&format!(") override {{
        return {}(({})this, ", 
        fun_ptr_var_str, ffi_class_name));
    for param in &method.params {
        c_class_callback_method_impl.push_str(&format!("({}){}, ", get_str_ffi_type(&param.field_type), param.name));
    }
    if !method.params.is_empty() {
        c_class_callback_method_impl.truncate(c_class_callback_method_impl.len() - ", ".len()); // 去掉最后一个参数的, 
    }
    c_class_callback_method_impl.push_str(");\n\t};\n");

    return c_class_callback_method_impl;
}

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
void {}_regist({} {});
",
        get_str_ffi_type(&method.return_type), fun_ptr_type_str, params_decl_str,
        fun_ptr_type_str, fun_ptr_type_str, method.name
    );

    // .cpp中的函数指针变量定义
    // 1. 注册函数指针的实现
    let regist_var_decl = format!("static {} {} = nullptr;\n", fun_ptr_type_str, fun_ptr_var_str);

    // .cpp中的注册函数实现
    let regist_impl = format!("void {}_regist({} {}){{
    {} = {};
}};
", fun_ptr_type_str, fun_ptr_type_str, method.name, fun_ptr_var_str, method.name);

    return (regist_decl, regist_var_decl, regist_impl);
}

fn get_str_ffi_type(field_type: &FieldType) -> String {
    match field_type.type_kind {
        TypeKind::Void | TypeKind::Int64 | TypeKind::Float | TypeKind::Double | TypeKind::Char => {
            return field_type.full_str.clone();
        }
        TypeKind::String => {
            return "const char*".to_string();
        }
        TypeKind::Class => {
            return format!("FFI_{}", field_type.type_str);
        }
        _ => {
            unimplemented!("get_ffi_type_str: unknown type kind");
        }
    }
}

fn get_str_method_decl(class: Option<&Class>, method: &Method) -> String {
    let ffi_decl_name = get_str_ffi_decl_class_name(class, method);
    let params = get_str_params_decl(class, method);
    let method_decl = format!("{} {}({});", 
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
            method_impl = format!("{} {{
    {}
    return ({})new {}({});
}};", method_prefix, param_prefix, impl_return_type, decl_class_name, param_str);
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
            let call_prefix = if decl_class_name.is_empty() { "" } else { "ptr->" };
            if method.return_type.type_kind == TypeKind::String {
                method_impl = format!("{} {{
    {}
    static std::string retStr = {}{}({});
    return (const char*)retStr.c_str();
}};", method_prefix, param_prefix, call_prefix, method.name, param_str);
            } 
            else if (method.return_type.type_kind == TypeKind::Class && 0 == method.return_type.ptr_level) {
                method_impl = format!("{} {{
    {}
    return ({})new {}({}{}({}));
}};", method_prefix, param_prefix, impl_return_type, method.return_type.type_str, call_prefix, method.name, param_str);
            }
            else {
                method_impl = format!("{} {{
    {}
    return ({}){}{}({});
}};", method_prefix, param_prefix, impl_return_type, call_prefix, method.name, param_str);
            }
        }
        _ => {
            unimplemented!("gen_c_class_method_impl: unknown method type");
        }
    }

    return method_impl
}

/// 函数是不是需要加第一个类的实例参数，模拟调用类实例的调用方法
pub fn get_is_need_first_class_param(class: Option<&Class>, method: &Method) -> bool {
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
        if method.method_type == MethodType::Destructor {
            param_strs.push("void* obj".to_string());
        } else {
            param_strs.push(format!("FFI_{} obj", get_str_decl_class_name(class, method)));
        }
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
            param_prefixs.push(format!("{}* ptr = ({}*)obj;", cur_class.type_str, cur_class.type_str));
        } else {
            unimplemented!("method_build_params_impl: need first class param but class is None");
        }
    }

    for param in &method.params {
        if param.field_type.type_kind == TypeKind::String {
            param_strs.push(format!("std::string({})", param.name));
        }
        else if (param.field_type.type_kind == TypeKind::Class && 0 == param.field_type.ptr_level) {
            param_strs.push(format!("({})(*({}*){})", &param.field_type.full_str, param.field_type.type_str, param.name));
        } 
        else {
            param_strs.push(format!("({}){}", &param.field_type.full_str, param.name));
        }
    }

    let param_prefixs_str = param_prefixs.join("\n");
    let param_strs_str = param_strs.join(", ");
    return (param_prefixs_str, param_strs_str);
}
