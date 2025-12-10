use std::{fs, io::Write, path::{Path, PathBuf}};

use crate::gen_context::*;

pub fn gen_c(gen_context: &GenContext, gen_out_dir: &str) {
    // 为每个文件生成对应的 FFI 文件
    for element in &gen_context.hpp_elements {
        match element {
            HppElement::File(file) => {
                gen_c_file(gen_context, file, gen_out_dir);
            }
            _ => {
                // 跳过非文件元素
            }
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

#ifdef __cplusplus
extern \"C\" {{
#endif
");
    // 收集所有需要生成 typedef 的类型名
    let mut typedef_names = vec![];

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
    collect_referenced_types(file, &mut typedef_names);

    // 3. 为所有收集到的类型生成 typedef
    for typedef_name in &typedef_names {
        ch_header.push_str(&format!("typedef void* FFI_{};\n", typedef_name));
    }

    ch_str.push_str(&ch_header);
    let mut cc_str = String::new();
    let cc_header = format!("
#include \"{}\"
#include <set>
#include <mutex>
#include <map>
#include <condition_variable>
#include <atomic>

extern \"C\" {{
#include \"dart_api_dl.h\"
}}

#include \"{}\"

extern \"C\" {{

", hpp_filename, h_filename);
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
            // Enum 类型
            HppElement::Enum(enum_def) => {
                gen_c_enum(&mut c_context, enum_def);
            }
            _ => {
                // clang 解析出现问题时，可能会产生一些预期外的元素
                // 跳过这些元素，避免程序崩溃
            }
        }
    }

    // 公共尾
    let ch_footer = r#"
#ifdef __cplusplus
} // extern "C"
#endif
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
    // 跳过无类名的方法（通常来自第三方库的模板实例化）
    // 这些方法的 FFI 名称会是 ffi__method_name（注意双下划线）
    let ffi_name = get_str_ffi_decl_class_name(class, method);
    if ffi_name.starts_with("ffi__") {
        return;
    }

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
    let mut regist_var_decl_global = String::new();  // 全局变量声明（异步回调）
    let mut regist_var_decl_member = String::new();  // 成员变量声明（同步回调）
    let mut regist_impl = String::new();
    let mut method_id_enums = String::new();  // 方法 ID 枚举
    let mut has_sync_callback = false;  // 是否有同步回调方法

    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                // 这次只生成需要重写的回调函数
                if method.method_type == MethodType::Normal {
                    let (local_regist_decl, local_regist_var_decl, local_regist_impl) = get_str_callback_method_regist(Some(&class), method);
                    regist_decl.push_str(&local_regist_decl);
                    // 根据是否需要同步调用，决定变量声明的位置
                    if callback_needs_sync_call(method) {
                        // 同步回调：helper函数放在全局（供_regist使用），成员变量放在类内
                        // local_regist_var_decl包含两部分：成员变量声明 + helper函数定义
                        // 需要拆分它们
                        let var_decl_lines: Vec<&str> = local_regist_var_decl.lines().collect();
                        let mut member_part = String::new();
                        let mut global_part = String::new();
                        let mut in_member_section = true;

                        for line in var_decl_lines {
                            // 成员变量声明行（第一行）
                            if line.contains("= nullptr") {
                                member_part.push_str(line);
                                member_part.push('\n');
                                in_member_section = false;
                            } else if !in_member_section {
                                // helper 函数定义（后续行）
                                global_part.push_str(line);
                                global_part.push('\n');
                            }
                        }

                        regist_var_decl_global.push_str(&global_part);
                        regist_var_decl_member.push_str(&member_part);
                        has_sync_callback = true;
                        // 生成方法 ID 常量
                        method_id_enums.push_str(&format!(
                            "static constexpr int64_t METHOD_ID_{}_{} = {};\n",
                            class.type_str,
                            method.name,
                            format!("0x{:016x}", {
                                use std::collections::hash_map::DefaultHasher;
                                use std::hash::{Hash, Hasher};
                                let mut hasher = DefaultHasher::new();
                                format!("{}_{}", class.type_str, method.name).hash(&mut hasher);
                                hasher.finish()
                            })
                        ));
                    } else {
                        // 异步回调：函数指针是全局变量
                        regist_var_decl_global.push_str(&local_regist_var_decl);
                    }
                    regist_impl.push_str(&local_regist_impl);
                }
            }
            HppElement::Field(_field) => {
                // 回调类的字段在注册阶段不需要特殊处理，会在后续统一生成 getter/setter
            }
            _ => {
                unimplemented!("gen_c_callback_class: unknown child");
            }
        }
    }

    // 如果有同步回调，生成全局的请求-响应管理器
    let request_response_manager = if has_sync_callback {
        format!("
// 全局请求-响应管理器（用于同步回调）
{}

static std::map<int64_t, int64_t> callback_results;  // request_id -> result
static std::mutex callback_results_mutex;
static std::condition_variable callback_results_cv;

// Dart 调用此函数来设置回调结果
API_EXPORT void FFI_{}_setCallbackResult(int64_t request_id, int64_t result) {{
    std::lock_guard<std::mutex> lock(callback_results_mutex);
    callback_results[request_id] = result;
    callback_results_cv.notify_all();
}}
", method_id_enums, class.type_str)
    } else {
        String::new()
    };

    // 在 .h 文件中添加 setCallbackResult 函数声明
    if has_sync_callback {
        c_context.ch_str.push_str(&format!("
API_EXPORT void FFI_{}_setCallbackResult(int64_t request_id, int64_t result);
", class.type_str));
    }

    // 生成回调子类
    let c_class_callback_impl = format!("{}{}
class {} : {} {{
public:
", regist_var_decl_global, request_response_manager, subclass_name, class.type_str);
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
            HppElement::Field(_field) => {
                // 回调子类继承父类的字段，不需要重新声明
            }
            _ => {
                unimplemented!("gen_c_callback_class: unknown child");
            }
        }
    }
    // 在类定义的末尾添加成员变量声明
    if !regist_var_decl_member.is_empty() {
        c_context.cc_str.push_str("\n    // 同步回调的函数指针成员变量\n");
        c_context.cc_str.push_str(&regist_var_decl_member);
    }
    c_context.cc_str.push_str("\n};\n");

    // 生成回调子类的其他正常函数和字段访问器
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
                // 为回调类的字段生成 getter 和 setter
                let (get_decl, set_decl) = get_str_field_decl(Some(&class), field);
                c_context.ch_str.push_str(&format!("{}\n", get_decl));
                c_context.ch_str.push_str(&format!("{}\n", set_decl));

                let (get_impl, set_impl) = get_str_field_impl(Some(&class), field);
                c_context.cc_str.push_str(&format!("{}\n", get_impl));
                c_context.cc_str.push_str(&format!("{}\n", set_impl));
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

/// 判断 callback 方法是否需要同步调用
/// 根据注释中的 @callback_sync 或 @callback_async 标记决定
/// 如果没有标记，默认为异步（使用 SendPort）
fn callback_needs_sync_call(method: &Method) -> bool {
    // 检查是否有明确的同步标记
    method.is_sync_callback
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

    // 根据返回值类型选择实现方式
    if callback_needs_sync_call(method) {
        return get_str_callback_method_impl_sync(class, method);
    } else {
        return get_str_callback_method_impl_async(class, method);
    }
}

/// 生成异步 callback 方法实现（void 返回值）
fn get_str_callback_method_impl_async(class: Option<&Class>, method: &Method) -> String {
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
        if param.field_type.type_kind == TypeKind::String {
            param_name = format!("{}.c_str()", param_name);
        }
        else if (param.field_type.type_kind == TypeKind::Class) && (param.field_type.ptr_level == 0) {
            param_name = format!("(new {}({}))", param.field_type.type_str, param_name);
        }
        else if param.field_type.type_kind == TypeKind::StdPtr || param.field_type.type_kind == TypeKind::StdVector {
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

    // 生成返回语句（异步回调无法立即返回值，返回默认值）
    let return_stmt = match method.return_type.type_kind {
        TypeKind::Void => String::new(),
        TypeKind::Int64 | TypeKind::Char => "\n        return 0;".to_string(),
        TypeKind::Bool => "\n        return false;".to_string(),
        TypeKind::Float => "\n        return 0.0f;".to_string(),
        TypeKind::Double => "\n        return 0.0;".to_string(),
        _ => "\n        return 0;".to_string(),
    };

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

        // 创建临时副本以避免在持有锁时调用 Dart API
        std::set<int64_t> callbackPorts;
        {{
            std::lock_guard<std::mutex> lock(get{}Mutex());
            callbackPorts = get{}Set();
        }}

        for (const auto& item : callbackPorts) {{
            Dart_PostCObject_DL((Dart_Port_DL)item, &args);
        }}{}
}};
",
        method.return_type.full_str, method.name, decl_params_str,
        gen_values_str,
        values_str,
        args_num,
        fun_ptr_var_str,
        fun_ptr_var_str,
        return_stmt,
    );

    return ret_str;
}

/// 生成同步 callback 方法实现（使用函数指针）
/// C++ 直接调用 Dart 函数指针，避免事件循环阻塞
fn get_str_callback_method_impl_sync(class: Option<&Class>, method: &Method) -> String {
    let class_name = class.unwrap().type_str.as_str();
    let method_name = &method.name;

    // 构造参数列表
    let mut decl_params = Vec::new();
    let mut call_params = Vec::new();
    let mut param_conversions = Vec::new();

    // 添加 this 指针作为第一个参数
    call_params.push("(int64_t)this".to_string());

    for (i, param) in method.params.iter().enumerate() {
        decl_params.push(format!("{} {}", param.field_type.full_str, param.name));

        // 转换参数为FFI类型
        let param_call = match param.field_type.type_kind {
            TypeKind::Int64 | TypeKind::Bool => {
                format!("(int64_t){}", param.name)
            }
            TypeKind::Float | TypeKind::Double => {
                // 生成临时变量来转换 float/double 到 int64_t
                let temp_var = format!("_param_{}", i);
                param_conversions.push(format!(
                    "        {} *_ptr_{} = ({} *)&{};\n        int64_t {} = *((int64_t *)_ptr_{});",
                    param.field_type.full_str, i, param.field_type.full_str, param.name, temp_var, i
                ));
                temp_var
            }
            TypeKind::String => {
                // String 需要转换为 const char* 指针
                format!("(int64_t){}.c_str()", param.name)
            }
            _ => format!("(int64_t){}", param.name)
        };
        call_params.push(param_call);
    }

    let decl_params_str = decl_params.join(", ");
    let call_params_str = call_params.join(", ");
    let param_conversions_str = if param_conversions.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", param_conversions.join("\n"))
    };

    // 生成默认返回值
    let default_return = match method.return_type.type_kind {
        TypeKind::Int64 => "0",
        TypeKind::Float | TypeKind::Double => "0.0",
        TypeKind::Bool => "false",
        _ => "0",
    };

    // 函数指针类型
    let fnptr_name = format!("{}_{}_fnptr", class_name, method_name);
    let return_cpp_type = &method.return_type.full_str;

    // 生成实现
    let ret_str = if method.return_type.type_kind == TypeKind::Void {
        // void 返回类型的特殊处理
        format!("    virtual void {}({}) override {{
        if ({} == nullptr) {{
            return;  // 没有注册函数指针
        }}
{}
        // 直接调用 Dart 函数指针
        {}({});
    }}
",
            method_name, decl_params_str,
            fnptr_name,
            param_conversions_str,
            fnptr_name, call_params_str
        )
    } else {
        // 非 void 返回类型
        format!("    virtual {} {}({}) override {{
        if ({} == nullptr) {{
            return {};  // 没有注册函数指针，返回默认值
        }}
{}
        // 直接调用 Dart 函数指针
        int64_t result = {}({});

        // 转换返回值
        {}
    }}
",
            return_cpp_type, method_name, decl_params_str,
            fnptr_name,
            default_return,
            param_conversions_str,
            fnptr_name, call_params_str,
            match method.return_type.type_kind {
                TypeKind::Int64 => "return (int)result;".to_string(),
                TypeKind::Bool => "return (bool)result;".to_string(),
                TypeKind::Float => "return *((float *)&result);".to_string(),
                TypeKind::Double => "return *((double *)&result);".to_string(),
                _ => "return result;".to_string(),
            }
        )
    };

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

    // 根据返回值类型选择注册方式
    if callback_needs_sync_call(method) {
        return get_str_callback_method_regist_sync(class, method);
    } else {
        return get_str_callback_method_regist_async(class, method);
    }
}

/// 生成异步回调的注册代码（void 返回值，使用 port）
fn get_str_callback_method_regist_async(class: Option<&Class>, method: &Method) -> (String, String, String) {
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
    // 使用函数内静态变量确保线程安全的初始化（Meyer's Singleton）
    let regist_var_decl = format!("// 使用函数内静态变量确保线程安全的初始化
static std::set<int64_t>& get{}Set() {{
    static std::set<int64_t> callbackSet;
    return callbackSet;
}}

static std::mutex& get{}Mutex() {{
    static std::mutex callbackMutex;
    return callbackMutex;
}}

", fun_ptr_var_str, fun_ptr_var_str);

    // .cpp中的注册函数实现
    let regist_impl = format!("API_EXPORT void {}_regist(int64_t {}){{
    // 参数校验：确保 port 有效（Dart port 通常不会是 0）
    if ({} == 0) {{
        return;
    }}
    
    // 使用互斥锁保护 set 的访问，防止多线程竞争导致内部结构损坏
    std::lock_guard<std::mutex> lock(get{}Mutex());
    get{}Set().insert({});
}};
", 
    fun_ptr_type_str, method.name, 
    method.name,
    fun_ptr_var_str,
    fun_ptr_var_str, method.name,
);

    return (regist_decl, regist_var_decl, regist_impl);
}

/// 生成同步回调的注册代码（有返回值，使用 SendPort）
fn get_str_callback_method_regist_sync(class: Option<&Class>, method: &Method) -> (String, String, String) {
    // ffi 中的类型名
    let ffi_class_name = format!("FFI_{}", class.unwrap().type_str);
    // 函数指针类型的名字
    let fun_ptr_type_str = format!("{}_{}_FnPtr", ffi_class_name, method.name);
    // 指向函数指针的变量
    let fun_ptr_var_str = format!("{}_{}_fnptr", class.unwrap().type_str, method.name);
    // Port 集合变量名（用于通过 SendPort 发送消息）
    let port_var_str = format!("{}_{}", class.unwrap().type_str, method.name);

    // 构造函数指针的参数列表（包含 this 指针和实际参数）
    // 注意：Dart 的 Pointer.fromFunction 要求所有参数都是 int64_t
    let mut fnptr_params = vec!["int64_t obj".to_string()];
    for (i, _param) in method.params.iter().enumerate() {
        fnptr_params.push(format!("int64_t param{}", i));
    }
    let fnptr_params_str = fnptr_params.join(", ");

    // 获取返回值的 C FFI 类型 - 也必须是 int64_t
    let return_type = "int64_t".to_string();

    // .h中的函数指针类型和注册函数定义
    let regist_decl = format!("typedef {} (*{})({});
API_EXPORT void {}_register(FFI_{} obj, {} fnptr);
API_EXPORT void {}_regist(int64_t {});
",
        return_type, fun_ptr_type_str, fnptr_params_str,
        fun_ptr_type_str, class.unwrap().type_str, fun_ptr_type_str,
        fun_ptr_type_str, method.name
    );

    // .cpp中的函数指针变量定义（包含成员变量和 Port 集合）
    let regist_var_decl = format!("    {} {} = nullptr;

// 使用函数内静态变量确保线程安全的初始化
static std::set<int64_t>& get{}Set() {{
    static std::set<int64_t> callbackSet;
    return callbackSet;
}}

static std::mutex& get{}Mutex() {{
    static std::mutex callbackMutex;
    return callbackMutex;
}}

", fun_ptr_type_str, fun_ptr_var_str, port_var_str, port_var_str);

    // .cpp中的注册函数实现
    let regist_impl = format!("API_EXPORT void {}_register(FFI_{} obj, {} fnptr) {{
    if (obj && fnptr) {{
        static_cast<Impl_{}*>(obj)->{} = fnptr;
    }}
}}
API_EXPORT void {}_regist(int64_t {}){{
    // 参数校验：确保 port 有效（Dart port 通常不会是 0）
    if ({} == 0) {{
        return;
    }}

    // 使用互斥锁保护 set 的访问，防止多线程竞争导致内部结构损坏
    std::lock_guard<std::mutex> lock(get{}Mutex());
    get{}Set().insert({});
}};
",
        fun_ptr_type_str, class.unwrap().type_str, fun_ptr_type_str,
        class.unwrap().type_str, fun_ptr_var_str,
        fun_ptr_type_str, method.name,
        method.name,
        port_var_str,
        port_var_str, method.name,
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
        TypeKind::Enum => {
            // 枚举类型在 C FFI 中使用 int 表示
            return "int".to_string();
        }
        TypeKind::String => {
            return "const char*".to_string();
        }
        TypeKind::Class => {
            // 清理类型名，移除const、&、*等修饰符
            let cleaned = field_type.type_str
                .replace("const ", "")
                .replace("const&", "")
                .replace("&", "")
                .replace("*", "")
                .replace(" ", "");
            let clean_type_str = cleaned.trim();
            return format!("FFI_{}", clean_type_str);
        }
        TypeKind::StdPtr => {
            // 清理类型名
            let cleaned = field_type.type_str
                .replace("const ", "")
                .replace("const&", "")
                .replace("&", "")
                .replace("*", "")
                .replace(" ", "");
            let clean_type_str = cleaned.trim();
            return format!("FFI_StdPtr_{}", clean_type_str);
        }
        TypeKind::StdVector => {
            if field_type.value_type.is_none() {
                return format!("FFI_StdVector_Unknown");
            }
            let value_type = field_type.value_type.as_deref().unwrap();
            if value_type.type_kind == TypeKind::String {
                return format!("FFI_StdVector_String");
            } else {
                return format!("FFI_StdVector_{}", field_type.get_value_type_str());
            }
        }
        TypeKind::StdMap => {
            // 如果模板参数解析失败，返回一个占位符类型
            if field_type.key_type.is_none() || field_type.value_type.is_none() {
                return format!("FFI_StdMap_Unknown");
            }

            let key_type = field_type.key_type.as_deref().unwrap();
            let value_type = field_type.value_type.as_deref().unwrap();

            let key_type_str = if key_type.type_kind == TypeKind::String {
                "String".to_string()
            } else {
                field_type.get_key_type_str()
            };
            
            let value_type_str = if value_type.type_kind == TypeKind::String {
                "String".to_string()
            } else {
                field_type.get_value_type_str()
            };
            
            return format!("FFI_StdMap_{}_{}", key_type_str, value_type_str);
        }
        TypeKind::StdUnorderedMap => {
            if field_type.key_type.is_none() || field_type.value_type.is_none() {
                return format!("FFI_StdUnorderedMap_Unknown");
            }
            let key_type = field_type.key_type.as_deref().unwrap();
            let value_type = field_type.value_type.as_deref().unwrap();

            let key_type_str = if key_type.type_kind == TypeKind::String {
                "String".to_string()
            } else {
                field_type.get_key_type_str()
            };

            let value_type_str = if value_type.type_kind == TypeKind::String {
                "String".to_string()
            } else {
                field_type.get_value_type_str()
            };

            return format!("FFI_StdUnorderedMap_{}_{}", key_type_str, value_type_str);
        }
        TypeKind::StdSet => {
            if field_type.value_type.is_none() {
                return format!("FFI_StdSet_Unknown");
            }
            let value_type = field_type.value_type.as_deref().unwrap();
            if value_type.type_kind == TypeKind::String {
                return format!("FFI_StdSet_String");
            } else {
                return format!("FFI_StdSet_{}", field_type.get_value_type_str());
            }
        }
        TypeKind::StdUnorderedSet => {
            if field_type.value_type.is_none() {
                return format!("FFI_StdUnorderedSet_Unknown");
            }
            let value_type = field_type.value_type.as_deref().unwrap();
            if value_type.type_kind == TypeKind::String {
                return format!("FFI_StdUnorderedSet_String");
            } else {
                return format!("FFI_StdUnorderedSet_{}", field_type.get_value_type_str());
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
            else if method.return_type.type_kind == TypeKind::StdMap {
                let container_type = method.return_type.full_str.clone();
                method_impl = format!("{} {{
    {}
    return ({})new {}({});
}};", method_prefix, param_prefix, impl_return_type, container_type, param_str);
            }
            else if method.return_type.type_kind == TypeKind::StdUnorderedMap {
                let container_type = method.return_type.full_str.clone();
                method_impl = format!("{} {{
    {}
    return ({})new {}({});
}};", method_prefix, param_prefix, impl_return_type, container_type, param_str);
            }
            else if method.return_type.type_kind == TypeKind::StdSet {
                // 确保使用正确的 std::set 类型
                let container_type = method.return_type.full_str.clone();
                method_impl = format!("{} {{
    {}
    return ({})new {}({});
}};", method_prefix, param_prefix, impl_return_type, container_type, param_str);
            }
            else if method.return_type.type_kind == TypeKind::StdUnorderedSet {
                // 确保使用正确的 std::unordered_set 类型
                let container_type = method.return_type.full_str.clone();
                method_impl = format!("{} {{
    {}
    return ({})new {}({});
}};", method_prefix, param_prefix, impl_return_type, container_type, param_str);
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
    
    // 特殊处理 Map 和 Set 的方法
    if let Some(cur_class) = class {
        if cur_class.class_type == ClassType::StdMap {
            match method_name {
                "insert" => {
                    // Map insert() 需要使用 std::make_pair
                    if param_str.is_some() {
                        let params: Vec<&str> = param_str.unwrap().split(", ").collect();
                        if params.len() == 2 {
                            return format!("return (void)ptr->insert(std::make_pair({}, {}));", params[0], params[1]);
                        }
                    }
                    return format!("return (void)ptr->insert{};", full_param_str);
                }
                "find" => {
                    // Map find() 返回迭代器，需要检查是否找到并返回值
                    if return_field_type.type_kind == TypeKind::String {
                        return format!("static std::string retStr = \"\";
    auto it = ptr->find{};
    if (it != ptr->end()) {{
        retStr = it->second;
    }} else {{
        retStr = \"\";
    }}
    return (const char*)retStr.c_str();", full_param_str);
                    } else {
                        return format!("auto it = ptr->find{};
    return ({})(it != ptr->end() ? it->second : {});", full_param_str, impl_return_type, 
                            if return_field_type.type_kind == TypeKind::Int64 { "0" } else { "0" });
                    }
                }
                _ => {}
            }
        } else if cur_class.class_type == ClassType::StdUnorderedMap {
            match method_name {
                "insert" => {
                    // UnorderedMap insert() 需要使用 std::make_pair
                    if param_str.is_some() {
                        let params: Vec<&str> = param_str.unwrap().split(", ").collect();
                        if params.len() == 2 {
                            return format!("return (void)ptr->insert(std::make_pair({}, {}));", params[0], params[1]);
                        }
                    }
                    return format!("return (void)ptr->insert{};", full_param_str);
                }
                "find" => {
                    // UnorderedMap find() 返回迭代器，需要检查是否找到并返回值
                    if return_field_type.type_kind == TypeKind::String {
                        return format!("static std::string retStr = \"\";
    auto it = ptr->find{};
    if (it != ptr->end()) {{
        retStr = it->second;
    }} else {{
        retStr = \"\";
    }}
    return (const char*)retStr.c_str();", full_param_str);
                    } else {
                        return format!("auto it = ptr->find{};
    return ({})(it != ptr->end() ? it->second : {});", full_param_str, impl_return_type, 
                            if return_field_type.type_kind == TypeKind::Int64 { "0" } else { "0" });
                    }
                }
                _ => {}
            }
        } else if cur_class.class_type == ClassType::StdSet {
            match method_name {
                "insert" => {
                    // Set insert() 直接插入值
                    return format!("return (void)ptr->insert{};", full_param_str);
                }
                "contains" => {
                    // Set contains() 检查是否包含
                    return format!("return (int)(ptr->find{} != ptr->end());", full_param_str);
                }
                _ => {}
            }
        } else if cur_class.class_type == ClassType::StdUnorderedSet {
            match method_name {
                "insert" => {
                    // UnorderedSet insert() 直接插入值
                    return format!("return (void)ptr->insert{};", full_param_str);
                }
                "contains" => {
                    // UnorderedSet contains() 检查是否包含
                    return format!("return (int)(ptr->find{} != ptr->end());", full_param_str);
                }
                _ => {}
            }
        }
    }
    
    if return_field_type.type_kind == TypeKind::String {
        return format!("static std::string retStr = \"\";
    retStr = {}{}{};
    return (const char*)retStr.c_str();", call_prefix, method_name, full_param_str);
    } 
    else if return_field_type.type_kind == TypeKind::Class && 0 == return_field_type.ptr_level {
        return format!("return ({})new {}({}{}{});", impl_return_type, return_field_type.type_str, call_prefix, method_name, full_param_str);
    }
    else if (return_field_type.type_kind == TypeKind::StdPtr && 0 == return_field_type.ptr_level) 
    || (return_field_type.type_kind == TypeKind::StdVector && 0 == return_field_type.ptr_level) 
    {
        return format!("return ({})new {}({}{}{});", impl_return_type, return_field_type.full_str, call_prefix, method_name, full_param_str);
    }
    else if (return_field_type.type_kind == TypeKind::StdMap && 0 == return_field_type.ptr_level) 
    || (return_field_type.type_kind == TypeKind::StdUnorderedMap && 0 == return_field_type.ptr_level)
    || (return_field_type.type_kind == TypeKind::StdSet && 0 == return_field_type.ptr_level) 
    || (return_field_type.type_kind == TypeKind::StdUnorderedSet && 0 == return_field_type.ptr_level) 
    {
        let container_type = return_field_type.full_str.clone();
        return format!("return ({})new {}({}{}{});", impl_return_type, container_type, call_prefix, method_name, full_param_str);
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
                if let Some(value_type) = cur_class.value_type.as_deref() {
                    let suffix = value_type.full_str.clone();
                    param_prefixs.push(format!("std::vector<{}>* ptr = (std::vector<{}>*)obj;", suffix, suffix));
                }
            }
            else if cur_class.class_type ==  ClassType::StdMap {
                if let (Some(key_type), Some(value_type)) = (cur_class.key_type.as_deref(), cur_class.value_type.as_deref()) {
                    let key_suffix = key_type.full_str.clone();
                    let value_suffix = value_type.full_str.clone();
                    param_prefixs.push(format!("std::map<{}, {}>* ptr = (std::map<{}, {}>*)obj;", key_suffix, value_suffix, key_suffix, value_suffix));
                }
            }
            else if cur_class.class_type ==  ClassType::StdUnorderedMap {
                if let (Some(key_type), Some(value_type)) = (cur_class.key_type.as_deref(), cur_class.value_type.as_deref()) {
                    let key_suffix = key_type.full_str.clone();
                    let value_suffix = value_type.full_str.clone();
                    param_prefixs.push(format!("std::unordered_map<{}, {}>* ptr = (std::unordered_map<{}, {}>*)obj;", key_suffix, value_suffix, key_suffix, value_suffix));
                }
            }
            else if cur_class.class_type ==  ClassType::StdSet {
                if let Some(value_type) = cur_class.value_type.as_deref() {
                    let suffix = value_type.full_str.clone();
                    param_prefixs.push(format!("std::set<{}>* ptr = (std::set<{}>*)obj;", suffix, suffix));
                }
            }
            else if cur_class.class_type ==  ClassType::StdUnorderedSet {
                if let Some(value_type) = cur_class.value_type.as_deref() {
                    let suffix = value_type.full_str.clone();
                    param_prefixs.push(format!("std::unordered_set<{}>* ptr = (std::unordered_set<{}>*)obj;", suffix, suffix));
                }
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
    else if field_type.type_kind == TypeKind::Class && 0 == field_type.ptr_level {
        return format!("({})(*({}*){})", &field_type.full_str, field_type.type_str, param_name);
    }
    else if (field_type.type_kind == TypeKind::StdPtr && 0 == field_type.ptr_level)
    || (field_type.type_kind == TypeKind::StdVector && 0 == field_type.ptr_level)
    {
        return format!("({})(*({}*){})", &field_type.full_str, &field_type.full_str, param_name);
    }
    else if field_type.type_kind == TypeKind::StdMap && 0 == field_type.ptr_level {
        let container_type = field_type.full_str.clone();
        return format!("({})(*({}*){})", container_type, container_type, param_name);
    }
    else if field_type.type_kind == TypeKind::StdUnorderedMap && 0 == field_type.ptr_level {
        let container_type = field_type.full_str.clone();
        return format!("({})(*({}*){})", container_type, container_type, param_name);
    }
    else if field_type.type_kind == TypeKind::StdSet && 0 == field_type.ptr_level {
        let container_type = field_type.full_str.clone();
        return format!("({})(*({}*){})", container_type, container_type, param_name);
    }
    else if field_type.type_kind == TypeKind::StdUnorderedSet && 0 == field_type.ptr_level {
        let container_type = field_type.full_str.clone();
        return format!("({})(*({}*){})", container_type, container_type, param_name);
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
fn collect_referenced_types(file: &File, typedef_names: &mut Vec<String>) {
    // 递归收集文件中所有元素引用的类型
    for child in &file.children {
        collect_element_referenced_types(&child, typedef_names);
    }
}

/// 递归处理HppElement，收集其中引用的所有类型
fn collect_element_referenced_types(element: &HppElement, typedef_names: &mut Vec<String>) {
    match element {
        HppElement::File(file) => {
            for child in &file.children {
                collect_element_referenced_types(child, typedef_names);
            }
        },
        HppElement::Class(class) => {
            // 收集类中所有子元素引用的类型
            for child in &class.children {
                collect_element_referenced_types(child, typedef_names);
            }
        },
        HppElement::Method(method) => {
            // 处理返回类型
            collect_field_type(&method.return_type, typedef_names);
            
            // 处理参数类型
            for param in &method.params {
                collect_field_type(&param.field_type, typedef_names);
            }
        },
        HppElement::Field(field) => {
            // 处理字段类型
            collect_field_type(&field.field_type, typedef_names);
        },
        HppElement::Enum(_enum) => {
            // Enum 不需要收集引用类型，它本身就是类型定义
        }
    }
}

/// 处理单个字段类型，收集需要的typedef
fn collect_field_type(field_type: &FieldType, typedef_names: &mut Vec<String>) {
    // 跳过被忽略的类型
    if field_type.type_kind == TypeKind::Ignored {
        return;
    }

    match field_type.type_kind {
        TypeKind::Class => {
            // 清理类型名，移除const、&、*等修饰符
            let cleaned = field_type.type_str
                .replace("const ", "")
                .replace("const&", "")
                .replace("&", "")
                .replace("*", "")
                .replace(" ", "");
            let clean_type_str = cleaned.trim().to_string();

            // 使用 should_ignore_type 检查是否应该忽略这个类型
            if !clean_type_str.is_empty()
                && !typedef_names.contains(&clean_type_str)
                && !crate::gen_context::should_ignore_type(&clean_type_str) {
                typedef_names.push(clean_type_str);
            }
        },
        TypeKind::StdPtr => {
            // 清理基类类型名
            let cleaned = field_type.type_str
                .replace("const ", "")
                .replace("const&", "")
                .replace("&", "")
                .replace("*", "")
                .replace(" ", "");
            let clean_type_str = cleaned.trim().to_string();

            // 使用 should_ignore_type 检查是否应该忽略这个类型
            if !clean_type_str.is_empty()
                && !typedef_names.contains(&clean_type_str)
                && !crate::gen_context::should_ignore_type(&clean_type_str) {
                typedef_names.push(clean_type_str.clone());
                let stdptr_typename = format!("StdPtr_{}", clean_type_str);
                if !typedef_names.contains(&stdptr_typename) {
                    typedef_names.push(stdptr_typename);
                }
            }
        },
        TypeKind::StdVector => {
            // 处理vector内部的值类型
            if let Some(value_type) = &field_type.value_type {
                collect_field_type(value_type, typedef_names);

                // 添加StdVector类型本身
                let value_type_str = field_type.get_value_type_str();
                let vector_type_str = format!("StdVector_{}", value_type_str);
                if !typedef_names.contains(&vector_type_str) {
                    typedef_names.push(vector_type_str);
                }
            }
        },
        TypeKind::StdMap => {
            // 处理map内部的键和值类型
            if let Some(key_type) = &field_type.key_type {
                collect_field_type(key_type, typedef_names);
            }

            if let Some(value_type) = &field_type.value_type {
                collect_field_type(value_type, typedef_names);

                // 添加StdMap类型本身
                let key_type_str = field_type.get_key_type_str();
                let value_type_str = field_type.get_value_type_str();
                let map_type_str = format!("StdMap_{}_{}", key_type_str, value_type_str);
                if !typedef_names.contains(&map_type_str) {
                    typedef_names.push(map_type_str);
                }
            }
        },
        TypeKind::StdUnorderedMap => {
            // 处理unordered_map内部的键和值类型
            if let Some(key_type) = &field_type.key_type {
                collect_field_type(key_type, typedef_names);
            }

            if let Some(value_type) = &field_type.value_type {
                collect_field_type(value_type, typedef_names);

                // 添加StdUnorderedMap类型本身
                let key_type_str = field_type.get_key_type_str();
                let value_type_str = field_type.get_value_type_str();
                let map_type_str = format!("StdUnorderedMap_{}_{}", key_type_str, value_type_str);
                if !typedef_names.contains(&map_type_str) {
                    typedef_names.push(map_type_str);
                }
            }
        },
        TypeKind::StdSet => {
            // 处理set内部的值类型
            if let Some(value_type) = &field_type.value_type {
                collect_field_type(value_type, typedef_names);

                // 添加StdSet类型本身
                let value_type_str = field_type.get_value_type_str();
                let set_type_str = format!("StdSet_{}", value_type_str);
                if !typedef_names.contains(&set_type_str) {
                    typedef_names.push(set_type_str);
                }
            }
        },
        TypeKind::StdUnorderedSet => {
            // 处理unordered_set内部的值类型
            if let Some(value_type) = &field_type.value_type {
                collect_field_type(value_type, typedef_names);

                // 添加StdUnorderedSet类型本身
                let value_type_str = field_type.get_value_type_str();
                let set_type_str = format!("StdUnorderedSet_{}", value_type_str);
                if !typedef_names.contains(&set_type_str) {
                    typedef_names.push(set_type_str);
                }
            }
        },
        _ => {} // 其他基本类型不需要特殊处理
    }
}

/// 为 enum 生成 C FFI 代码
fn gen_c_enum(_c_context: &mut CFileContext, enum_def: &Enum) {
    // Enum 在 C++ 层面就是整数类型，不需要生成额外的 FFI 函数
    // 只需要确保 C++ 头文件中有 enum 定义即可
    //
    // 注意：由于我们生成的 _ffi.h 文件会被 _ffi.cpp 包含，
    // 而 _ffi.cpp 又会包含原始的 .hpp 文件，
    // 所以 enum 定义会自动可用，无需在此生成
    //
    // 如果将来需要在 FFI 层做类型检查或转换，可以在这里添加

    // 暂时不生成任何代码
    // 可以在这里添加注释说明这个 enum 已被处理
    let _ = enum_def; // 标记为已使用，避免编译警告
}
