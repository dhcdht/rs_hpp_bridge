use std::{fs, io::Write, path::{Path, PathBuf}};

use crate::{gen_c, gen_context::*};

pub fn gen_dart(gen_context: &GenContext, gen_out_dir: &str) {
    gen_dart_public(gen_context, gen_out_dir);

    for hpp_element in &gen_context.hpp_elements {
        gen_dart_api(gen_context, hpp_element, gen_out_dir, None);
        gen_dart_fun(gen_context, hpp_element, gen_out_dir, None);
    }
}

#[derive(Debug, Default)]
struct DartGenContext<'a> {
    pub cur_file: Option<fs::File>,
    pub cur_class: Option<&'a Class>,
}

fn gen_dart_public<'a>(gen_context: &GenContext, gen_out_dir: &str) {
    let public_file_name = format!("{}_public.dart", gen_context.module_name);
    let public_file_path = PathBuf::new().join(gen_out_dir).join(public_file_name.clone()).into_os_string().into_string().unwrap();
    let public_file_str = format!("
import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

late final DynamicLibrary {}_dylib;
void {}_setDylib(DynamicLibrary dylib) {{
    {}_dylib = dylib;
    return;
}}

late final ptr_ffi_Dart_InitializeApiDL = {}_dylib.lookup<NativeFunction<Int64 Function(Pointer<Void>)>>('Dart_InitializeApiDL');
late final ffi_Dart_InitializeApiDL = ptr_ffi_Dart_InitializeApiDL.asFunction<int Function(Pointer<Void>)>();
    ", gen_context.module_name,
    gen_context.module_name,
    gen_context.module_name,
    gen_context.module_name,
    );

    let mut public_file = fs::File::create(public_file_path).unwrap();
    public_file.write_all(public_file_str.as_bytes());
}

fn gen_dart_fun<'a>(gen_context: &GenContext, hpp_element: &'a HppElement, gen_out_dir: &str, dart_gen_context: Option<&mut DartGenContext<'a>>) {
    match hpp_element {
        HppElement::File(file) => {
            let mut dart_gen_context = DartGenContext::default();

            let hpp_filename = Path::new(&file.path).file_name().unwrap().to_os_string().into_string().unwrap();
            let filename_without_ext = match hpp_filename.rfind(".") {
                Some(idx) => &hpp_filename[..idx],
                None => &hpp_filename,
            };
            let dart_ffiapi_filename = format!("{}_ffiapi.dart", filename_without_ext);
            let dart_filename = format!("{}.dart", filename_without_ext);
            let dart_path = PathBuf::new().join(gen_out_dir).join(dart_filename.clone()).into_os_string().into_string().unwrap();
            let mut dart_file = fs::File::create(dart_path).unwrap();

            // 收集当前文件中所有引用的外部类型
            let mut referenced_types = Vec::new();
            collect_referenced_types_from_file(file, &mut referenced_types);
            
            // 生成导入语句，使用 HashSet 去重
            let mut import_set = std::collections::HashSet::new();
            for type_name in &referenced_types {
                // 为每个引用的类型生成对应的import语句
                // 需要检查类型来源于哪个文件，这里做简化处理
                if let Some(import_file) = find_type_source_file(gen_context, type_name, filename_without_ext) {
                    import_set.insert(format!("import '{}.dart';", import_file));
                }
            }
            
            // 将去重后的 import 语句排序并拼接
            let mut import_statements = String::new();
            let mut sorted_imports: Vec<_> = import_set.into_iter().collect();
            sorted_imports.sort();
            for import_stmt in sorted_imports {
                import_statements.push_str(&format!("{}\n", import_stmt));
            }

            // 公共头
            let file_header = format!("
import '{}';
import 'dart:ffi';
import 'package:ffi/ffi.dart';
import 'dart:isolate';
{}            \n", dart_ffiapi_filename, import_statements);
            dart_file.write(file_header.as_bytes());

            dart_gen_context.cur_file = Some(dart_file);
            for hpp_element in &file.children {
                gen_dart_fun(gen_context, hpp_element, gen_out_dir, Some(&mut dart_gen_context));
            }
        }
        HppElement::Class(class) => {
            let local_dart_gen_context = dart_gen_context.unwrap();

            // 公共头
            let dart_file_header = local_dart_gen_context.cur_file.as_mut().unwrap();
            let mut class_header = format!("
{}
class {} implements Finalizable {{
    late Pointer<Void> _nativePtr;
    Pointer<Void> getNativePtr() {{
        return _nativePtr;
    }}
    static final _finalizer = NativeFinalizer(ptr_ffi_{}_Destructor);

    /**
     * dart对象释放时，释放native对象，默认行为
     */
    void nativeLifecycleLink() {{
        _finalizer.attach(this, _nativePtr, detach: this);
    }}
    /**
     * dart对象释放时，不释放native对象
     */
    void nativeLifecycleUnlink() {{
        _finalizer.detach(this);
    }}
", 
            class.comment_str.as_ref().unwrap_or(&"".to_string()),
            class.type_str, class.type_str);
            class_header.push_str(&format!("
    {}.FromNative(Pointer<Void> nativePtr) : _nativePtr = nativePtr {{}}
            \n", class.type_str));
            dart_file_header.write(class_header.as_bytes());

            // 回调类的特殊内容
            if class.is_callback() {
                let callback_header = format!("    static Map<Pointer<Void>, WeakReference<{}>> nativeToObjMap = {{}};\n\n", class.type_str);
                dart_file_header.write(callback_header.as_bytes());
            }
            
            local_dart_gen_context.cur_class = Some(class);
            for hpp_element in &class.children {
                gen_dart_fun(gen_context, hpp_element, gen_out_dir, Some(local_dart_gen_context));
            }
            local_dart_gen_context.cur_class = None;

            // 为StdMap、StdUnorderedMap和StdSet类添加便利方法
            if class.class_type == ClassType::StdMap {
                let dart_file = local_dart_gen_context.cur_file.as_mut().unwrap();
                let convenience_methods = generate_stdmap_convenience_methods(class);
                dart_file.write(convenience_methods.as_bytes());
            } else if class.class_type == ClassType::StdUnorderedMap {
                let dart_file = local_dart_gen_context.cur_file.as_mut().unwrap();
                let convenience_methods = generate_stdunorderedmap_convenience_methods(class);
                dart_file.write(convenience_methods.as_bytes());
            } else if class.class_type == ClassType::StdSet {
                let dart_file = local_dart_gen_context.cur_file.as_mut().unwrap();
                let convenience_methods = generate_stdset_convenience_methods(class);
                dart_file.write(convenience_methods.as_bytes());
            } else if class.class_type == ClassType::StdUnorderedSet {
                let dart_file = local_dart_gen_context.cur_file.as_mut().unwrap();
                let convenience_methods = generate_stdunorderedset_convenience_methods(class);
                dart_file.write(convenience_methods.as_bytes());
            }

            // 回调类的静态包装函数（必须在类结束之前生成）
            let mut init_str = "".to_string();
            if class.is_callback() {
                local_dart_gen_context.cur_class = Some(class);
                for hpp_element in &class.children {
                    gen_dart_fun_for_regist_callback(gen_context, hpp_element, gen_out_dir, Some(local_dart_gen_context), &mut init_str);
                }
                local_dart_gen_context.cur_class = None;
            }

            {
            // 公共尾
            let dart_file_footer = local_dart_gen_context.cur_file.as_mut().unwrap();
            let class_footer = format!("}}\n\n");
            dart_file_footer.write(class_footer.as_bytes());
            }

            // 回调类的 init 函数（在类结束之后生成）
            if class.is_callback() {
                // 用于注册dart函数实现的函数
                let callback_footer = format!("\nvoid _{}_init() {{\n{}\n}}\n", class.type_str, init_str);
                let dart_file_footer = local_dart_gen_context.cur_file.as_mut().unwrap();
                dart_file_footer.write(callback_footer.as_bytes());
            }
        }
        HppElement::Method(method) => {
            let local_dart_gen_context = dart_gen_context.unwrap();
            let dart_file = local_dart_gen_context.cur_file.as_mut().unwrap();

            // 跳过无类名的方法（通常来自第三方库的模板实例化）
            // 这些方法的 FFI 名称会是 ffi__method_name（注意双下划线）
            let mut cur_class_name = "";
            if let Some(cur_class) = local_dart_gen_context.cur_class {
                cur_class_name = &cur_class.type_str;
            }
            let ffi_name = format!("ffi_{}_{}", cur_class_name, method.name);
            if ffi_name.starts_with("ffi__") {
                return;
            }

            let method_impl = get_str_dart_fun(local_dart_gen_context.cur_class, method);
            dart_file.write(method_impl.as_bytes());
        }
        HppElement::Field(field) => {
            let local_dart_gen_context = dart_gen_context.unwrap();
            let dart_file = local_dart_gen_context.cur_file.as_mut().unwrap();

            // 跳过无类名的字段 getter/setter（通常来自第三方库）
            let mut cur_class_name = "";
            if let Some(cur_class) = local_dart_gen_context.cur_class {
                cur_class_name = &cur_class.type_str;
            }
            let ffi_get_name = format!("ffi_{}_get_{}", cur_class_name, field.name);
            if ffi_get_name.starts_with("ffi__") {
                return;
            }

            // get
            let get_method = Method::new_get_for_field(field);
            let get_method_str = get_str_dart_fun(local_dart_gen_context.cur_class, &get_method);
            // set
            let set_method = Method::new_set_for_field(field);
            let set_method_str = get_str_dart_fun(local_dart_gen_context.cur_class, &set_method);
            dart_file.write(format!("{}\n{}\n", get_method_str, set_method_str).as_bytes());
        }
        HppElement::Enum(enum_def) => {
            let local_dart_gen_context = dart_gen_context.unwrap();
            let dart_file = local_dart_gen_context.cur_file.as_mut().unwrap();

            let enum_code = gen_dart_enum(enum_def);
            dart_file.write(enum_code.as_bytes());
        }
        _ => {
            unimplemented!("gen_dart_api: unknown child");
        }
    }
}

fn gen_dart_api<'a>(gen_context: &GenContext, hpp_element: &'a HppElement, gen_out_dir: &str, ffiapi_gen_context: Option<&mut DartGenContext<'a>>) {
    match hpp_element {
        HppElement::File(file) => {
            let mut ffiapi_gen_context = DartGenContext::default();

            let hpp_filename = Path::new(&file.path).file_name().unwrap().to_os_string().into_string().unwrap();
            let filename_without_ext = match hpp_filename.rfind(".") {
                Some(idx) => &hpp_filename[..idx],
                None => &hpp_filename,
            };
            let dart_ffiapi_filename = format!("{}_ffiapi.dart", filename_without_ext);
            let dart_ffiapi_path = PathBuf::new().join(gen_out_dir).join(dart_ffiapi_filename.clone()).into_os_string().into_string().unwrap();
            let mut ffiapi_file = fs::File::create(dart_ffiapi_path).unwrap();

            let public_file_name = format!("{}_public.dart", gen_context.module_name);
            // 公共头
            let file_header = format!("
import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';
import '{}';
            \n", public_file_name);
            ffiapi_file.write(file_header.as_bytes());

            ffiapi_gen_context.cur_file = Some(ffiapi_file);
            for hpp_element in &file.children {
                gen_dart_api(gen_context, hpp_element, gen_out_dir, Some(&mut ffiapi_gen_context));
            }
        }
        HppElement::Class(class) => {
            let local_ffiapi_gen_context = ffiapi_gen_context.unwrap();

            local_ffiapi_gen_context.cur_class = Some(class);

            // 如果是回调类且有同步回调（有返回值的方法），生成 setCallbackResult FFI 绑定
            if class.is_callback() {
                let has_sync_callback = class.children.iter().any(|child| {
                    if let HppElement::Method(m) = child {
                        m.method_type == MethodType::Normal && m.return_type.type_kind != TypeKind::Void
                    } else {
                        false
                    }
                });

                if has_sync_callback {
                    let ffiapi_file = local_ffiapi_gen_context.cur_file.as_mut().unwrap();
                    let set_result_api = format!("late final ptr_ffi_FFI_{}_setCallbackResult = {}_dylib.lookup<NativeFunction<Void Function(Int64, Int64)>>('FFI_{}_setCallbackResult');
late final ffi_FFI_{}_setCallbackResult = ptr_ffi_FFI_{}_setCallbackResult.asFunction<void Function(int, int)>();
",
                        class.type_str, gen_context.module_name, class.type_str,
                        class.type_str, class.type_str);
                    ffiapi_file.write(set_result_api.as_bytes());
                }
            }

            for hpp_element in &class.children {
                gen_dart_api(gen_context, hpp_element, gen_out_dir, Some(local_ffiapi_gen_context));
            }
            local_ffiapi_gen_context.cur_class = None;
        }
        HppElement::Method(method) => {
            let local_ffiapi_gen_context = ffiapi_gen_context.unwrap();
            let ffiapi_file = local_ffiapi_gen_context.cur_file.as_mut().unwrap();

            // 跳过无类名的方法（通常来自第三方库的模板实例化）
            // 这些方法的 FFI 名称会是 ffi__method_name（注意双下划线）
            let mut cur_class_name = "";
            if let Some(cur_class) = local_ffiapi_gen_context.cur_class {
                cur_class_name = &cur_class.type_str;
            }
            let ffi_name = format!("ffi_{}_{}", cur_class_name, method.name);
            if ffi_name.starts_with("ffi__") {
                return;
            }

            if local_ffiapi_gen_context.cur_class.is_some() && local_ffiapi_gen_context.cur_class.unwrap().is_callback() {
                // 对于回调类，需要特殊生成注册函数
                let dart_api_str = get_str_dart_api_for_regist_callback(gen_context, local_ffiapi_gen_context.cur_class, method);
                ffiapi_file.write(format!("{}", dart_api_str).as_bytes());
            }

            let dart_api_str = get_str_dart_api(gen_context, local_ffiapi_gen_context.cur_class, method);
            ffiapi_file.write(format!("{}\n", dart_api_str).as_bytes());
        }
        HppElement::Field(field) => {
            let local_ffiapi_gen_context = ffiapi_gen_context.unwrap();
            let ffiapi_file = local_ffiapi_gen_context.cur_file.as_mut().unwrap();

            // 跳过无类名的字段 getter/setter（通常来自第三方库）
            let mut cur_class_name = "";
            if let Some(cur_class) = local_ffiapi_gen_context.cur_class {
                cur_class_name = &cur_class.type_str;
            }
            let ffi_get_name = format!("ffi_{}_get_{}", cur_class_name, field.name);
            if ffi_get_name.starts_with("ffi__") {
                return;
            }

            // get
            let get_method = Method::new_get_for_field(field);
            let get_method_str = get_str_dart_api(gen_context, local_ffiapi_gen_context.cur_class, &get_method);
            // set
            let set_method = Method::new_set_for_field(field);
            let set_method_str = get_str_dart_api(gen_context, local_ffiapi_gen_context.cur_class, &set_method);
            ffiapi_file.write(format!("{}\n{}\n", get_method_str, set_method_str).as_bytes());
        }
        HppElement::Enum(_enum_def) => {
            // Enum 不需要生成 FFI API，因为它们就是整数类型
            // 在 Dart 层已经通过 enum 类定义处理了类型转换
        }
        _ => {
            unimplemented!("gen_dart_ffiapi: unknown child");
        }
    }
}

/// 为回调类生成特殊的内容, init_str 用于出实话注册的内容
fn gen_dart_fun_for_regist_callback<'a>(gen_context: &GenContext, hpp_element: &'a HppElement, gen_out_dir: &str, dart_gen_context: Option<&mut DartGenContext<'a>>, init_str: &mut String) {
    match hpp_element {
        HppElement::Method(method) => {
            let local_dart_gen_context = dart_gen_context.unwrap();
            let dart_file = local_dart_gen_context.cur_file.as_mut().unwrap();

            let (local_init_str, dart_fun_impl) = get_dart_fun_for_regist_callback(local_dart_gen_context.cur_class, method);
            init_str.push_str(&local_init_str);
            dart_file.write(dart_fun_impl.as_bytes());
        }
        _ => {
            unimplemented!("gen_dart_api_for_callback_fun: unknown child");
        }
    }
}

fn get_str_dart_fun(class: Option<&Class>, method: &Method) -> String {
    // 独立函数和类的函数，都走下边逻辑，需要注意区分
    let (cur_class_name, class_is_callback) = if let Some(cur_class) = class {
        (cur_class.type_str.as_str(), cur_class.is_callback())
    } else {
        ("", false)
    };

    let callbck_block = get_str_dart_fun_callback_block(class, method);
    let params_decl_str = get_str_dart_fun_params_decl(class, method);
    let fun_body = if class_is_callback {
        get_str_dart_fun_body_for_callback(class, method)
    } else {
        get_str_dart_fun_body(class, method)
    };

    let mut fun_name = "".to_string();
    let static_modifier = if method.is_static { "static " } else { "" };
    match method.method_type {
        MethodType::Normal | MethodType::Destructor => {
            fun_name.push_str(&format!("{}{} {}", static_modifier, get_str_dart_fun_type(&method.return_type), method.name));
        }
        MethodType::Constructor => {
            fun_name.push_str(&format!("{}.{}", cur_class_name, method.name));
        }
        _ => {
            unimplemented!("gen_dart_api: unknown method type")
        }
    }

    let dart_fun_impl = format!("    {}
    {}({}) {{
        {}
    }}
{}",
        method.comment_str.as_ref().unwrap_or(&"".to_string()),
        fun_name, params_decl_str,
        fun_body,
        callbck_block,
    );

    return dart_fun_impl;
}

fn get_str_dart_fun_body(class: Option<&Class>, method: &Method) -> String {
    // 独立函数和类的函数，都走下边逻辑，需要注意区分
    let (cur_class_name, class_is_callback) = if let Some(cur_class) = class {
        (cur_class.type_str.as_str(), cur_class.is_callback())
    } else {
        ("", false)
    };
    let ffiapi_c_method_name = format!("ffi_{}_{}", cur_class_name, method.name);
    let params_str = get_str_dart_fun_params_impl(class, method);

    // 仅对非回调类的方法（包括普通/构造/析构）处理字符串参数内存释放
    let mut string_params: Vec<String> = Vec::new();
    if !class_is_callback {
        for param in &method.params {
            if param.field_type.type_kind == TypeKind::String {
                string_params.push(param.name.clone());
            }
        }
    }

    let mut body_prefix = "".to_string();
    let mut body_suffix = "".to_string();
    match method.method_type {
        MethodType::Normal => {
            if method.return_type.type_kind == TypeKind::Class {
                body_prefix.push_str(&format!("return {}.FromNative({}(", get_str_dart_fun_type(&method.return_type), ffiapi_c_method_name));
                body_suffix.push_str("));");
            }
            else if (method.return_type.type_kind == TypeKind::StdPtr)
            || method.return_type.type_kind == TypeKind::StdVector
            || method.return_type.type_kind == TypeKind::StdMap
            || method.return_type.type_kind == TypeKind::StdUnorderedMap
            || method.return_type.type_kind == TypeKind::StdSet
            || method.return_type.type_kind == TypeKind::StdUnorderedSet
            {
                body_prefix.push_str(&format!("return {}.FromNative({}(", get_str_dart_fun_type(&method.return_type), ffiapi_c_method_name));
                body_suffix.push_str("));");
            }
            else {
                body_prefix.push_str(&format!("return {}(", ffiapi_c_method_name));
                if method.return_type.type_kind == TypeKind::String {
                    body_suffix.push_str(").toDartString();");
                } else if method.return_type.type_kind == TypeKind::Enum {
                    // 枚举类型：从 int 转换为枚举，使用 fromValue() 方法
                    body_suffix.push_str(&format!("))!;"));
                    // 修改 prefix 以包含 fromValue 调用
                    body_prefix = format!("return {}.fromValue({}(", get_str_dart_fun_type(&method.return_type), ffiapi_c_method_name);
                } else {
                    body_suffix.push_str(");");
                }
            }
        }
        MethodType::Constructor => {
            body_prefix.push_str(&format!("_nativePtr = {}(", ffiapi_c_method_name));
            body_suffix.push_str(");
        nativeLifecycleLink();");
            if class.unwrap().class_type == ClassType::StdPtr {
                body_suffix.push_str("
        // stdptr 会接管 obj 对象的生命周期，所以这里不需要 obj 对象再跟 native 对象绑定了
        obj.nativeLifecycleUnlink();");
            }
        }
        MethodType::Destructor => {
            body_prefix.push_str(&format!("nativeLifecycleUnlink();\n\t\treturn {}(", ffiapi_c_method_name));
            body_suffix.push_str(");");
        }
    }

    let core_body = format!("{}{}{}", body_prefix, params_str, body_suffix);

    // 如果没有字符串参数，保持原样
    if string_params.is_empty() {
        return core_body;
    }

    // 有字符串参数：生成 _c_param 变量、try/finally 释放
    // params_str 中针对字符串参数会使用占位符 _c_<name>
    let alloc_lines: Vec<String> = string_params.iter().map(|n| format!("final _c_{} = {}.toNativeUtf8();", n, n)).collect();
    let free_lines: Vec<String> = string_params.iter().map(|n| format!("malloc.free(_c_{});", n)).collect();
    // 保持最小侵入：不改变 core_body 内容，仅包裹
    let wrapped = format!("{}
        try {{
            {}
        }} finally {{
            {}
        }}",
        alloc_lines.join("\n\t\t"),
        core_body,
        free_lines.join("\n\t\t\t")
    );

    return wrapped;
}

fn get_str_dart_fun_callback_block(class: Option<&Class>, method: &Method) -> String {
    if method.method_type != MethodType::Normal {
        return "".to_string();
    }
    // 独立函数和类的函数，都走下边逻辑，需要注意区分
    let (cur_class_name, class_is_callback) = if let Some(cur_class) = class {
        (cur_class.type_str.as_str(), cur_class.is_callback())
    } else {
        ("", false)
    };
    if cur_class_name.is_empty() || !class_is_callback {
        return "".to_string();
    }

    // 判断是否需要同步调用（使用 @callback_sync 注释标记）
    let needs_sync_call = method.is_sync_callback;

    if needs_sync_call {
        // 同步 callback（使用函数指针）：生成静态回调函数和 block
        let params_str = get_str_dart_fun_params_decl(class, method);

        // 为同步回调只生成 block 定义，不生成 ReceivePort
        // 静态函数会在 _MyCallback_init 中使用 Pointer.fromFunction 生成
        let block_str = format!("    {} Function({})? {}_block = null;",
            get_str_dart_fun_type(&method.return_type), params_str, method.name,
        );
        return block_str;
    } else {
        // 异步 callback（void 返回值）：生成 ReceivePort 和 block
        let port_args_str = get_str_port_fun_params_impl(class, method);
        let params_str = get_str_dart_fun_params_decl(class, method);
        let block_str = format!("    static final {}_port = ReceivePort()..listen((data) {{
        final args = data as List;
        final nativePtr = Pointer<Void>.fromAddress(args[0]);
        final obj = {}.nativeToObjMap[nativePtr]?.target;
        obj?.{}({});
    }});
    {} Function({})? {}_block = null;",
            method.name,
            cur_class_name,
            method.name, port_args_str,
            get_str_dart_fun_type(&method.return_type), params_str, method.name,
        );
        return block_str;
    }
}

fn get_str_port_fun_params_impl(class: Option<&Class>, method: &Method) -> String {
    // 独立函数和类的函数，都走下边逻辑，需要注意区分
    let class_is_callback = if let Some(cur_class) = class {
        cur_class.is_callback()
    } else {
        false
    };
    let is_destructor = method.method_type == MethodType::Destructor;

    let mut param_strs = Vec::new();
    if (gen_c::get_is_need_first_class_param(class, method) && !class_is_callback) || is_destructor {
        param_strs.push("_nativePtr".to_string());
    }
    for i in 0..method.params.len() {
        let index = i+1;
        let param = &method.params[i];
        if param.field_type.type_kind == TypeKind::Class {
            param_strs.push(format!("{}.FromNative(Pointer<Void>.fromAddress(args[{}]))", get_str_dart_fun_type(&param.field_type), index));
        }
        else if param.field_type.type_kind == TypeKind::StdPtr
        || param.field_type.type_kind == TypeKind::StdVector
        {
            param_strs.push(format!("{}.FromNative(Pointer<Void>.fromAddress(args[{}]))", get_str_dart_fun_type(&param.field_type), index));
        }
        else if param.field_type.type_kind == TypeKind::Char
        {
            param_strs.push(format!{"(args[{}] as String).toNativeUtf8().cast()", index});
        }
        else {
            param_strs.push(format!("args[{}]", index));
        }
    }

    return param_strs.join(", ");
}

/// 为同步回调生成参数解析代码（从 args[3] 开始，因为前3个是 request_id, method_id, this）
fn get_str_port_fun_params_impl_for_sync_callback(class: Option<&Class>, method: &Method) -> String {
    let mut param_strs = Vec::new();
    for i in 0..method.params.len() {
        let index = i+3;  // 前3个是 request_id, method_id, this
        let param = &method.params[i];
        if param.field_type.type_kind == TypeKind::Class {
            param_strs.push(format!("{}.FromNative(Pointer<Void>.fromAddress(args[{}]))", get_str_dart_fun_type(&param.field_type), index));
        }
        else if param.field_type.type_kind == TypeKind::StdPtr
        || param.field_type.type_kind == TypeKind::StdVector
        {
            param_strs.push(format!("{}.FromNative(Pointer<Void>.fromAddress(args[{}]))", get_str_dart_fun_type(&param.field_type), index));
        }
        else if param.field_type.type_kind == TypeKind::Char
        {
            param_strs.push(format!{"(args[{}] as String).toNativeUtf8().cast()", index});
        }
        else {
            param_strs.push(format!("args[{}]", index));
        }
    }

    return param_strs.join(", ");
}

fn get_str_dart_fun_body_for_callback(class: Option<&Class>, method: &Method) -> String {
    // 独立函数和类的函数，都走下边逻辑，需要注意区分
    let (cur_class_name, class_is_callback) = if let Some(cur_class) = class {
        (cur_class.type_str.as_str(), cur_class.is_callback())
    } else {
        ("", false)
    };
    let ffiapi_c_method_name = format!("ffi_{}_{}", cur_class_name, method.name);
    let params_str = get_str_dart_fun_params_impl(class, method);

    let exception_default_value_str = get_str_dart_api_exception_default_value(&method.return_type);
    let exception_value_str = if exception_default_value_str.is_empty() {
        "".to_string()
    } else {
        format!(" ?? {}", exception_default_value_str)
    };
    let mut body_prefix = "".to_string();
    let mut body_suffix = "".to_string();
    match method.method_type {
        MethodType::Normal => {
            body_prefix.push_str(&format!("return {}_block?.call(", method.name));
            body_suffix.push_str(&format!("){};", exception_value_str));
        }
        MethodType::Constructor => {
            body_prefix.push_str(&format!("_nativePtr = {}(", ffiapi_c_method_name));

            // 为同步回调方法生成 Pointer.fromFunction 注册代码
            let mut sync_callback_registrations = String::new();
            if let Some(cur_class) = class {
                if cur_class.is_callback() {
                    for child in &cur_class.children {
                        if let HppElement::Method(m) = child {
                            if m.method_type == MethodType::Normal && m.is_sync_callback {
                                // 生成函数指针类型签名
                                let mut ffi_type_params = vec!["Int64".to_string()]; // this 指针
                                for param in &m.params {
                                    ffi_type_params.push("Int64".to_string()); // 所有参数都是 int64
                                }
                                let ffi_signature = format!("Int64 Function({})", ffi_type_params.join(", "));

                                // 生成默认返回值（必须是编译时常量）
                                // 对于所有类型都使用 0，在 static wrapper 中进行适当的类型转换
                                let default_value = "0".to_string();

                                sync_callback_registrations.push_str(&format!("\n        final {}_ptr = Pointer.fromFunction<{}>({}._{}_{}_static, {});",
                                    m.name, ffi_signature, cur_class_name, cur_class_name, m.name, default_value));
                                sync_callback_registrations.push_str(&format!("\n        FFI_{}_{}_FnPtr_register(_nativePtr, {}_ptr);",
                                    cur_class_name, m.name, m.name));
                            }
                        }
                    }
                }
            }

            body_suffix.push_str(&format!(");
        nativeLifecycleLink();
        nativeToObjMap[_nativePtr] = WeakReference<{}>(this);{}
        _{}_init();", cur_class_name, sync_callback_registrations, cur_class_name));
        }
        MethodType::Destructor => {
            body_prefix.push_str(&format!("nativeLifecycleUnlink();\n\t\treturn {}(", ffiapi_c_method_name));
            body_suffix.push_str(");");
        }
        _ => {
            unimplemented!("gen_dart_api: unknown method type")
        }
    }

    let fun_body = format!("{}{}{}",
        body_prefix, params_str, body_suffix);

    return fun_body;
}

// 已删除 get_str_dart_ffi_params_decl_for_sync_callback 和 get_str_dart_sync_callback_wrapper
// 同步回调现在通过 ReceivePort 和 setCallbackResult 实现，不再需要 Pointer.fromFunction

fn get_str_dart_fun_params_decl(class: Option<&Class>, method: &Method) -> String {
    let mut param_strs = Vec::new();
    for param in &method.params {
        param_strs.push(format!("{} {}", get_str_dart_fun_type(&param.field_type), param.name));
    }

    return param_strs.join(", ");
}

fn get_str_dart_fun_params_impl(class: Option<&Class>, method: &Method) -> String {
    // 独立函数和类的函数，都走下边逻辑，需要注意区分
    let class_is_callback = if let Some(cur_class) = class {
        cur_class.is_callback()
    } else {
        false
    };
    let is_destructor = method.method_type == MethodType::Destructor;

    let mut param_strs = Vec::new();
    if (gen_c::get_is_need_first_class_param(class, method) && !class_is_callback) || is_destructor {
        param_strs.push("_nativePtr".to_string());
    }
    for param in &method.params {
        if class_is_callback {
            param_strs.push(format!("{}", param.name));
            continue;
        }

        if !class_is_callback && param.field_type.type_kind == TypeKind::Class {
            param_strs.push(format!("{}.getNativePtr()", param.name));
        }
        else if param.field_type.type_kind == TypeKind::StdPtr
        || param.field_type.type_kind == TypeKind::StdVector
        || param.field_type.type_kind == TypeKind::StdMap
        || param.field_type.type_kind == TypeKind::StdUnorderedMap
        || param.field_type.type_kind == TypeKind::StdSet
        || param.field_type.type_kind == TypeKind::StdUnorderedSet
        {
            param_strs.push(format!("{}.getNativePtr()", param.name));
        }
        else if param.field_type.type_kind == TypeKind::String {
            // 使用占位符变量，实际分配在 get_str_dart_fun_body 中完成
            param_strs.push(format!("_c_{}", param.name));
        }
        else if param.field_type.type_kind == TypeKind::Enum {
            // 枚举类型需要访问 .value 属性来获取整数值
            param_strs.push(format!("{}.value", param.name));
        }
        else {
            param_strs.push(format!("{}", param.name));
        }
    }

    return param_strs.join(", ");
}

/// (初始化内容，回调函数的实现内容)
fn get_dart_fun_for_regist_callback(class: Option<&Class>, method: &Method) -> (String, String) {
    if method.method_type != MethodType::Normal {
        return ("".to_string(), "".to_string());
    }

    // 独立函数和类的函数，都走下边逻辑，需要注意区分
    let cur_class_name = if let Some(cur_class) = class {
        &cur_class.type_str
    } else {
        ""
    };

    // 判断是否需要同步调用（使用 is_sync_callback 标志）
    let needs_sync_call = method.is_sync_callback;

    if needs_sync_call {
        // 同步 callback（使用函数指针）：生成静态包装函数和注册代码
        let dart_callback_fun_name = format!("_{}_{}_static", cur_class_name, method.name);

        // 为每个同步回调方法生成一个静态包装函数
        // 这个函数签名必须匹配 C++ 的函数指针签名：(int64 obj, int64 param1, ...)
        let mut ffi_param_strs = vec!["int objPtr".to_string()];  // this 指针作为第一个参数
        let mut call_param_strs = Vec::new();

        for param in &method.params {
            // FFI 层所有参数都是 int (int64_t 映射)
            ffi_param_strs.push(format!("int {}_raw", param.name));

            // 根据类型转换参数
            let converted_param = match param.field_type.type_kind {
                TypeKind::Int64 | TypeKind::Char | TypeKind::Bool => {
                    format!("{}_raw", param.name)
                }
                TypeKind::Float => {
                    // 从 int64 位模式重新解释为 float
                    format!("(() {{ final p = malloc<Int64>(); p.value = {}_raw; final f = p.cast<Float>().value; malloc.free(p); return f; }})()", param.name)
                }
                TypeKind::Double => {
                    // 从 int64 位模式重新解释为 double
                    format!("(() {{ final p = malloc<Int64>(); p.value = {}_raw; final d = p.cast<Double>().value; malloc.free(p); return d; }})()", param.name)
                }
                TypeKind::String => {
                    // String 通过指针传递，需要转换
                    format!("Pointer<Utf8>.fromAddress({}_raw).toDartString()", param.name)
                }
                _ => format!("{}_raw", param.name),
            };
            call_param_strs.push(converted_param);
        }

        let ffi_params_str = ffi_param_strs.join(", ");
        let call_params_str = call_param_strs.join(", ");

        // 生成默认返回值
        let default_return = match method.return_type.type_kind {
            TypeKind::Int64 => "0",
            TypeKind::Float | TypeKind::Double => "0.0",
            TypeKind::Bool => "false",
            _ => "0",
        };

        // 生成返回值转换代码和函数体
        let (callback_body, return_stmt) = match method.return_type.type_kind {
            TypeKind::Void => {
                // void 返回：不需要 result 变量，直接调用函数
                let body = format!("      obj.{}_block!({});", method.name, call_params_str);
                let ret = "return 0;".to_string(); // FFI 函数必须返回 int64
                (body, ret)
            }
            TypeKind::Int64 | TypeKind::Char => {
                let body = format!("      final result = obj.{}_block!({});", method.name, call_params_str);
                let ret = "return result;".to_string();
                (body, ret)
            }
            TypeKind::Bool => {
                let body = format!("      final result = obj.{}_block!({});", method.name, call_params_str);
                let ret = "return (result ? 1 : 0);".to_string();
                (body, ret)
            }
            TypeKind::Float => {
                let body = format!("      final result = obj.{}_block!({});", method.name, call_params_str);
                let ret = "return (() { final p = malloc<Float>(); p.value = result; final i = p.cast<Int64>().value; malloc.free(p); return i; })();".to_string();
                (body, ret)
            }
            TypeKind::Double => {
                let body = format!("      final result = obj.{}_block!({});", method.name, call_params_str);
                let ret = "return (() { final p = malloc<Double>(); p.value = result; final i = p.cast<Int64>().value; malloc.free(p); return i; })();".to_string();
                (body, ret)
            }
            _ => {
                let body = format!("      final result = obj.{}_block!({});", method.name, call_params_str);
                let ret = "return result;".to_string();
                (body, ret)
            }
        };

        // 生成静态包装函数
        let dart_fun_impl = format!("
  // 同步回调静态包装函数（供 C++ 通过函数指针调用）
  static int {}({}) {{
    final nativePtr = Pointer<Void>.fromAddress(objPtr);
    final obj = {}.nativeToObjMap[nativePtr]?.target;

    if (obj != null && obj.{}_block != null) {{
{}
      {}
    }}
    return 0;  // 默认返回值
  }}
",
            dart_callback_fun_name, ffi_params_str,
            cur_class_name,
            method.name,
            callback_body,
            return_stmt
        );

        // 生成注册代码：使用 Pointer.fromFunction 创建函数指针并注册到 C++
        let register_fun_name = format!("FFI_{}_{}_FnPtr_register", cur_class_name, method.name);
        let init_str = format!("
    // 为每个对象注册同步回调函数指针
    // 注意：这在构造函数中完成，见 get_str_dart_fun_body_for_callback
");

        return (init_str, dart_fun_impl);
    } else {
        // 异步 callback（void 返回值）：使用原有的 port 注册逻辑
        // native函数指针类型的名字
        let native_fun_type_name = format!("FFI_{}_{}", cur_class_name, method.name);
        // 注册函数的名字
        let native_regist_fun_name = format!("{}_regist", native_fun_type_name);
        // 实现函数的名字
        let dart_callback_fun_name = format!("_{}_{}", cur_class_name, method.name);
        let params_decl_str = get_str_dart_fun_params_decl_for_regist_callback(class, method);
        let params_impl_str = get_str_dart_fun_params_impl_for_regist_callback(class, method);

        // 生成用于初始化的内容
        let exception_default_value_str = get_str_dart_api_exception_default_value(&method.return_type);
        let exception_value_str = if exception_default_value_str.is_empty() {
            "".to_string()
        } else {
            format!(", {}", exception_default_value_str)
        };
        let init_str = format!("
    {{
    {}({}.{}_port.sendPort.nativePort);
    }}
",
            native_regist_fun_name, cur_class_name, method.name,
        );

        // 生成dart回调函数内容
        let dart_fun_impl = format!("{} {}({}) {{
    return {}.nativeToObjMap[native]!.target!.{}({});
}}
",
            get_str_dart_api_type(&method.return_type), dart_callback_fun_name, params_decl_str,
            cur_class_name, method.name, params_impl_str,
        );

        return (init_str, dart_fun_impl);
    }
}

fn get_str_dart_fun_params_decl_for_regist_callback(class: Option<&Class>, method: &Method) -> String {
    let mut param_strs = Vec::new();
    if gen_c::get_is_need_first_class_param(class, method) {
        param_strs.push("Pointer<Void> native".to_string());
    }
    for param in &method.params {
        param_strs.push(format!("{} {}", get_str_dart_api_type(&param.field_type), param.name));
    }

    return param_strs.join(", ");
}

fn get_str_dart_fun_params_impl_for_regist_callback(class: Option<&Class>, method: &Method) -> String {
    let mut param_strs = Vec::new();
    for param in &method.params {
        if param.field_type.type_kind == TypeKind::Class {
            param_strs.push(format!("{}.FromNative({})", param.field_type.type_str, param.name));
        } else if param.field_type.type_kind == TypeKind::String {
            // String parameters come as Pointer<Utf8>, need to convert to Dart String
            param_strs.push(format!("{}.toDartString()", param.name));
        } else if param.field_type.type_kind == TypeKind::StdVector
            || param.field_type.type_kind == TypeKind::StdMap
            || param.field_type.type_kind == TypeKind::StdUnorderedMap
            || param.field_type.type_kind == TypeKind::StdSet
            || param.field_type.type_kind == TypeKind::StdUnorderedSet {
            // STL containers need to be wrapped with FromNative
            param_strs.push(format!("{}.FromNative({})", get_str_dart_fun_type(&param.field_type), param.name));
        } else {
            param_strs.push(format!("{}", param.name));
        }
    }

    return param_strs.join(", ");
}

fn get_str_dart_fun_type(field_type: &FieldType) -> String {
    // 枚举类型，返回枚举类型名称
    if field_type.type_kind == TypeKind::Enum {
        return field_type.type_str.clone();
    }
    // class类型，需要对应 dart class
    else if field_type.type_kind == TypeKind::Class {
        // 清理C++语法，移除const、&、*等修饰符
        let clean_type = field_type.type_str
            .replace("const ", "")
            .replace("const&", "")
            .replace("&", "")
            .replace("*", "")
            .replace(" ", "")
            .replace("::", "");

        // Special handling for string types that might be misclassified as Class
        if clean_type == "stdstring" || clean_type == "string" {
            return "String".to_string();
        }

        return clean_type;
    }
    // 智能指针类型，需要对应 dart class
    else if field_type.type_kind == TypeKind::StdPtr {
        return format!("StdPtr_{}", field_type.type_str);
    }
    else if field_type.type_kind == TypeKind::StdVector {
        if let Some(value_type) = field_type.value_type.as_ref() {
            return format!("StdVector_{}", value_type.type_str);
        } else {
            return "StdVector_Unknown".to_string();
        }
    }
    else if field_type.type_kind == TypeKind::StdMap {
        if let (Some(key_type), Some(value_type)) = (field_type.key_type.as_ref(), field_type.value_type.as_ref()) {
            return format!("StdMap_{}_{}", key_type.type_str, value_type.type_str);
        } else {
            return "StdMap_Unknown".to_string();
        }
    }
    else if field_type.type_kind == TypeKind::StdUnorderedMap {
        if let (Some(key_type), Some(value_type)) = (field_type.key_type.as_ref(), field_type.value_type.as_ref()) {
            return format!("StdUnorderedMap_{}_{}", key_type.type_str, value_type.type_str);
        } else {
            return "StdUnorderedMap_Unknown".to_string();
        }
    }
    else if field_type.type_kind == TypeKind::StdSet {
        if let Some(value_type) = field_type.value_type.as_ref() {
            return format!("StdSet_{}", value_type.type_str);
        } else {
            return "StdSet_Unknown".to_string();
        }
    }
    else if field_type.type_kind == TypeKind::StdUnorderedSet {
        if let Some(value_type) = field_type.value_type.as_ref() {
            return format!("StdUnorderedSet_{}", value_type.type_str);
        } else {
            return "StdUnorderedSet_Unknown".to_string();
        }
    }

    // 基础数据类型
    if field_type.ptr_level == 0 {
        if field_type.type_kind == TypeKind::String {
            return "String".to_string();
        } else {
            return get_str_dart_api_type(field_type);
        }
    }

    // 基础类型的指针
    return get_str_native_api_type(field_type);
}

fn get_str_dart_api(gen_context: &GenContext, class: Option<&Class>, method: &Method) -> String {
    // 独立函数和类的函数，都走下边逻辑，需要注意区分
    let mut cur_class_name = "";
    if let Some(cur_class) = class {
        cur_class_name = &cur_class.type_str;
    }
    let ffiapi_c_method_name = format!("ffi_{}_{}", cur_class_name, method.name);
    let native_api_params_str = get_str_native_api_params_decl(class, method);
    let dart_api_params_str = get_str_dart_api_params_decl(class, method);

    let dar_api_str = format!("late final ptr_{} = {}_dylib.lookup<NativeFunction<{} Function({})>>('{}');
late final {} = ptr_{}.asFunction<{} Function({})>();
",
        ffiapi_c_method_name, gen_context.module_name, get_str_native_api_type(&method.return_type), native_api_params_str, ffiapi_c_method_name,
        ffiapi_c_method_name, ffiapi_c_method_name, get_str_dart_api_type(&method.return_type), dart_api_params_str,
    );
    return dar_api_str;
}

fn get_str_dart_api_for_regist_callback(gen_context: &GenContext, class: Option<&Class>, method: &Method) -> String {
    if method.method_type != MethodType::Normal {
        return "".to_string();
    }

    // 独立函数和类的函数，都走下边逻辑，需要注意区分
    let mut cur_class_name = "";
    if let Some(cur_class) = class {
        cur_class_name = &cur_class.type_str;
    }

    // 判断是否需要同步调用（使用 is_sync_callback 标志）
    let needs_sync_call = method.is_sync_callback;

    if needs_sync_call {
        // 同步 callback（使用函数指针）：生成 _register 和 _regist 函数的 FFI API 绑定
        // 函数指针类型名
        let native_fun_type_name = format!("FFI_{}_{}_FnPtr", cur_class_name, method.name);

        // 构造函数指针签名（Int64 Function(Int64, Int64, ...)）
        let mut ffi_type_params = vec!["Int64".to_string()]; // this 指针
        for _param in &method.params {
            ffi_type_params.push("Int64".to_string()); // 所有参数都是 int64
        }
        let ffi_signature = format!("Int64 Function({})", ffi_type_params.join(", "));

        // 生成 _register 函数的 FFI API 绑定（接受函数指针）
        let register_fun_name = format!("{}_register", native_fun_type_name);
        let register_api = format!("late final ptr_{} = {}_dylib.lookup<NativeFunction<Void Function(Pointer<Void>, Pointer<NativeFunction<{}>>)>>('{}');
late final {} = ptr_{}.asFunction<void Function(Pointer<Void>, Pointer<NativeFunction<{}>>)>();
",
            register_fun_name, gen_context.module_name, ffi_signature, register_fun_name,
            register_fun_name, register_fun_name, ffi_signature,
        );

        // 生成 _regist 函数的 FFI API 绑定（注册 SendPort，用于异步发送请求）
        let regist_fun_name = format!("{}_regist", native_fun_type_name);
        let regist_api = format!("late final ptr_{} = {}_dylib.lookup<NativeFunction<Void Function(Int64)>>('{}');
late final {} = ptr_{}.asFunction<void Function(int)>();
",
            regist_fun_name, gen_context.module_name, regist_fun_name,
            regist_fun_name, regist_fun_name,
        );

        return format!("{}{}", register_api, regist_api);
    } else {
        // 异步 callback（void 返回值）：使用原有的 _regist 逻辑
        // native函数指针类型的名字
        let native_fun_type_name = format!("FFI_{}_{}", cur_class_name, method.name);
        // 注册函数的名字
        let native_regist_fun_name = format!("{}_regist", native_fun_type_name);
        // 参数列表
        let params_str = get_str_native_api_params_decl(class, method);

        let dart_api_str = format!("typedef {} = {} Function({});
late final ptr_{} = {}_dylib.lookup<NativeFunction<Void Function(Int64)>>('{}');
late final {} = ptr_{}.asFunction<void Function(int)>();
",
            native_fun_type_name, get_str_native_api_type(&method.return_type), params_str,
            native_regist_fun_name, gen_context.module_name, native_regist_fun_name,
            native_regist_fun_name, native_regist_fun_name,
        );

        return dart_api_str;
    }
}

/// 生成 native callback 函数签名（用于 FFI API 中的 NativeFunction 类型）
/// 例如：Int64 Function(Pointer<Void>, Int64, Int64)
fn get_str_native_callback_function_signature(class: Option<&Class>, method: &Method) -> String {
    let mut param_strs = vec!["Pointer<Void>".to_string()];

    // 添加方法参数
    for param in &method.params {
        param_strs.push(get_str_native_api_type(&param.field_type));
    }

    format!("{} Function({})",
        get_str_native_api_type(&method.return_type),
        param_strs.join(", ")
    )
}

/// 返回dart api中的参数列表
fn get_str_dart_api_params_decl(class: Option<&Class>, method: &Method) -> String {
    let mut param_strs = Vec::new();
    if gen_c::get_is_need_first_class_param(class, method) {
        param_strs.push("Pointer<Void>".to_string());
    }
    for param in &method.params {
        param_strs.push(format!("{}", get_str_dart_api_type(&param.field_type)));
    }

    return param_strs.join(", ");
}

fn get_str_dart_api_type(field_type: &FieldType) -> String {
    // 基础数据类型
    if field_type.ptr_level == 0 {
        match field_type.type_kind {
            TypeKind::Void => {
                return "void".to_string();
            }
            TypeKind::Int64 => {
                return "int".to_string();
            }
            TypeKind::Float => {
                return "double".to_string();
            }
            TypeKind::Double => {
                return "double".to_string();
            }
            TypeKind::Char => {
                return "int".to_string();
            }
            TypeKind::Bool => {
                return "bool".to_string();
            }
            TypeKind::Enum => {
                // 枚举类型在 Dart FFI 中使用 int 表示
                return "int".to_string();
            }
            TypeKind::String => {
                return "Pointer<Utf8>".to_string();
            }
            TypeKind::Class => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdPtr => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdVector => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdMap => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdSet => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdUnorderedSet => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdUnorderedMap => {
                return "Pointer<Void>".to_string();
            }
            _ => {
                unimplemented!("get_dart_fun_type_str: unknown type kind, {:?}", field_type);
            }
        }
    }
    
    // class指针
    if field_type.type_kind == TypeKind::Class {
        return "Pointer<Void>".to_string();
    }

    // 基础类型的指针
    return get_str_native_api_type(field_type);
}

/// Pointer.fromFunction 对于有返回值的函数，必须有个默认值，否则无法编译
fn get_str_dart_api_exception_default_value(field_type: &FieldType) -> String {
    // 基础数据类型
    if field_type.ptr_level == 0 {
        match field_type.type_kind {
            TypeKind::Void => {
                return "".to_string();
            }
            TypeKind::Int64 => {
                return "0".to_string();
            }
            TypeKind::Float => {
                return "0.0".to_string();
            }
            TypeKind::Double => {
                return "0.0".to_string();
            }
            TypeKind::Char => {
                return "0".to_string();
            }
            TypeKind::Bool => {
                return "false".to_string();
            }
            TypeKind::Class => {
                return "nullptr".to_string();
            }
            _ => {
                unimplemented!("get_dart_fun_type_str: unknown type kind, {:?}", field_type);
            }
        }
    }
    // class指针
    // 基础类型的指针
    return "0".to_string();
}

/// 返回native api中的参数列表
fn get_str_native_api_params_decl(class: Option<&Class>, method: &Method) -> String {
    let mut param_strs = Vec::new();
    if gen_c::get_is_need_first_class_param(class, method) {
        param_strs.push("Pointer<Void>".to_string());
    }
    for param in &method.params {
        param_strs.push(format!("{}", get_str_native_api_type(&param.field_type)));
    }

    return param_strs.join(", ");
}

fn get_str_native_api_type(field_type: &FieldType) -> String {
    // 基础数据类型
    if field_type.ptr_level == 0 {
        match field_type.type_kind {
            TypeKind::Void => {
                return "Void".to_string();
            }
            TypeKind::Int64 => {
                return "Int64".to_string();
            }
            TypeKind::Float => {
                return "Float".to_string();
            }
            TypeKind::Double => {
                return "Double".to_string();
            }
            TypeKind::Char => {
                return "Int8".to_string();
            }
            TypeKind::Bool => {
                return "Bool".to_string();
            }
            TypeKind::Enum => {
                // 枚举类型在 Native API 中使用 Int64 表示
                return "Int64".to_string();
            }
            TypeKind::String => {
                return "Pointer<Utf8>".to_string();
            }
            TypeKind::Class => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdPtr => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdVector => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdMap => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdSet => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdUnorderedSet => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::StdUnorderedMap => {
                return "Pointer<Void>".to_string();
            }
            TypeKind::Ignored => {
                // 被忽略的类型不应该出现在公开 API 中
                // 如果出现了，说明有方法使用了不应该暴露的类型
                panic!("Ignored type '{}' found in method signature. This type should not be exposed in the API.", field_type.type_str);
            }
            _ => {
                unimplemented!("get_native_fun_type_str: unknown type kind, {:?}", field_type);
            }
        }
    }
    // class指针
    if field_type.type_kind == TypeKind::Class {
        return "Pointer<Void>".to_string();
    }

    // 基础类型的指针
    let base_native = match field_type.type_kind {
        TypeKind::Void => "Void",
        TypeKind::Int64 => "Int64",
        TypeKind::Float => "Float",
        TypeKind::Double => "Double",
        TypeKind::Char => "Int8",
        _ => unimplemented!("get_native_fun_type_str: unknown type kind in pointer"),
    };
    let mut native_type = base_native.to_string();
    for _ in 0..field_type.ptr_level {
        native_type = format!("Pointer<{}>", native_type);
    }
    return native_type
}

/// 收集文件中所有引用的外部类型
fn collect_referenced_types_from_file(file: &File, referenced_types: &mut Vec<String>) {
    for child in &file.children {
        collect_referenced_types_from_element(child, referenced_types);
    }
}

/// 递归收集元素中引用的类型
fn collect_referenced_types_from_element(element: &HppElement, referenced_types: &mut Vec<String>) {
    match element {
        HppElement::Class(class) => {
            for child in &class.children {
                collect_referenced_types_from_element(child, referenced_types);
            }
        },
        HppElement::Method(method) => {
            // 收集返回类型
            collect_referenced_types_from_field_type(&method.return_type, referenced_types);
            // 收集参数类型
            for param in &method.params {
                collect_referenced_types_from_field_type(&param.field_type, referenced_types);
            }
        },
        HppElement::Field(field) => {
            collect_referenced_types_from_field_type(&field.field_type, referenced_types);
        },
        _ => {}
    }
}

/// 从字段类型中收集引用的类型
fn collect_referenced_types_from_field_type(field_type: &FieldType, referenced_types: &mut Vec<String>) {
    match field_type.type_kind {
        TypeKind::Class => {
            let clean_type = field_type.type_str
                .replace("const ", "")
                .replace("const&", "")
                .replace("&", "")
                .replace("*", "")
                .replace(" ", "")
                .replace("::", "");
            
            // 排除string类型和基本类型
            if clean_type != "stdstring" && clean_type != "string" && clean_type != "" && !referenced_types.contains(&clean_type) {
                referenced_types.push(clean_type);
            }
        },
        TypeKind::StdPtr => {
            let ptr_type = format!("StdPtr_{}", field_type.type_str);
            if !referenced_types.contains(&ptr_type) {
                referenced_types.push(ptr_type);
            }
            // 也收集基础类型
            if !referenced_types.contains(&field_type.type_str) {
                referenced_types.push(field_type.type_str.clone());
            }
        },
        TypeKind::StdVector => {
            if let Some(value_type) = &field_type.value_type {
                let vector_type = format!("StdVector_{}", value_type.type_str);
                if !referenced_types.contains(&vector_type) {
                    referenced_types.push(vector_type);
                }
                // 递归收集值类型
                collect_referenced_types_from_field_type(value_type, referenced_types);
            }
        },
        TypeKind::StdMap => {
            if let Some(key_type) = &field_type.key_type {
                if let Some(value_type) = &field_type.value_type {
                    let map_type = format!("StdMap_{}_{}", key_type.type_str, value_type.type_str);
                    if !referenced_types.contains(&map_type) {
                        referenced_types.push(map_type);
                    }
                    // 递归收集键类型和值类型
                    collect_referenced_types_from_field_type(key_type, referenced_types);
                    collect_referenced_types_from_field_type(value_type, referenced_types);
                }
            }
        },
        TypeKind::StdSet => {
            if let Some(value_type) = &field_type.value_type {
                let set_type = format!("StdSet_{}", value_type.type_str);
                if !referenced_types.contains(&set_type) {
                    referenced_types.push(set_type);
                }
                // 递归收集值类型
                collect_referenced_types_from_field_type(value_type, referenced_types);
            }
        },
        TypeKind::StdUnorderedSet => {
            if let Some(value_type) = &field_type.value_type {
                let set_type = format!("StdUnorderedSet_{}", value_type.type_str);
                if !referenced_types.contains(&set_type) {
                    referenced_types.push(set_type);
                }
                // 递归收集值类型
                collect_referenced_types_from_field_type(value_type, referenced_types);
            }
        },
        _ => {} // 基本类型不需要处理
    }
}

/// 查找类型定义在哪个源文件中
fn find_type_source_file(gen_context: &GenContext, type_name: &str, current_file: &str) -> Option<String> {
    for hpp_element in &gen_context.hpp_elements {
        if let HppElement::File(file) = hpp_element {
            let file_path = Path::new(&file.path);
            let file_stem = file_path.file_stem()?.to_str()?;
            
            // 如果是当前文件，跳过
            if file_stem == current_file {
                continue;
            }
            
            // 检查文件中是否定义了这个类型
            if file_contains_type(file, type_name) {
                return Some(file_stem.to_string());
            }
        }
    }
    None
}

/// 检查文件中是否包含指定类型的定义
fn file_contains_type(file: &File, type_name: &str) -> bool {
    for child in &file.children {
        if element_contains_type(child, type_name) {
            return true;
        }
    }
    false
}

/// 检查元素中是否包含指定类型的定义
fn element_contains_type(element: &HppElement, type_name: &str) -> bool {
    match element {
        HppElement::Class(class) => {
            // 检查类名是否匹配
            if class.type_str == type_name {
                return true;
            }
            // 检查StdPtr和StdVector生成的类型
            if type_name.starts_with("StdPtr_") && format!("StdPtr_{}", class.type_str) == type_name {
                return true;
            }
            if type_name.starts_with("StdVector_") && format!("StdVector_{}", class.type_str) == type_name {
                return true;
            }
            // 递归检查子元素
            for child in &class.children {
                if element_contains_type(child, type_name) {
                    return true;
                }
            }
        },
        HppElement::Field(_) | HppElement::Method(_) => {
            // 字段和方法不定义类型，只引用类型
            return false;
        },
        HppElement::Enum(enum_def) => {
            // 检查 enum 名称是否匹配
            return enum_def.name == type_name;
        },
        HppElement::File(file) => {
            // 递归检查文件中的子元素
            for child in &file.children {
                if element_contains_type(child, type_name) {
                    return true;
                }
            }
        }
    }
    false
}

/// 为StdMap类生成便利方法
fn generate_stdmap_convenience_methods(class: &Class) -> String {
    // 如果模板参数解析失败，不生成便利方法
    let Some(key_type) = class.key_type.as_ref() else { return String::new(); };
    let Some(value_type) = class.value_type.as_ref() else { return String::new(); };
    let key_dart_type = get_str_dart_fun_type(key_type);
    let value_dart_type = get_str_dart_fun_type(value_type);
    
    format!(r#"
    // 便利构造函数 - 从Dart Map创建
    {}.fromMap(Map<{}, {}> map) {{
        _nativePtr = ffi_{}_Constructor();
        nativeLifecycleLink();
        for (var entry in map.entries) {{
            insert(entry.key, entry.value);
        }}
    }}
    
    // length属性
    int get length => size();
    
    // []操作符
    {} operator [](dynamic key) {{
        return find(key);
    }}
    
    // []= 操作符
    void operator []=(dynamic key, dynamic value) {{
        insert(key, value);
    }}
    
    // contains方法
    bool contains(dynamic key) {{
        return count(key) > 0;
    }}
    
    // 转换为Dart Map
    Map<{}, {}> toMap() {{
        Map<{}, {}> result = {{}};
        // 注意：这里需要通过FFI迭代来实现
        // 暂时返回空Map，需要额外的FFI支持来实现迭代
        return result;
    }}
"#, 
        class.type_str, 
        key_dart_type, value_dart_type,
        class.type_str,
        value_dart_type,
        key_dart_type, value_dart_type,
        key_dart_type, value_dart_type
    )
}

/// 为StdUnorderedMap类生成便利方法
fn generate_stdunorderedmap_convenience_methods(class: &Class) -> String {
    let Some(key_type) = class.key_type.as_ref() else { return String::new(); };
    let Some(value_type) = class.value_type.as_ref() else { return String::new(); };
    let key_dart_type = get_str_dart_fun_type(key_type);
    let value_dart_type = get_str_dart_fun_type(value_type);
    
    format!(r#"
    // 便利构造函数 - 从Dart Map创建
    {}.fromMap(Map<{}, {}> map) {{
        _nativePtr = ffi_{}_Constructor();
        nativeLifecycleLink();
        for (var entry in map.entries) {{
            insert(entry.key, entry.value);
        }}
    }}
    
    // length属性
    int get length => size();
    
    // []操作符
    {} operator [](dynamic key) {{
        return find(key);
    }}
    
    // []= 操作符
    void operator []=(dynamic key, dynamic value) {{
        insert(key, value);
    }}
    
    // contains方法
    bool contains(dynamic key) {{
        return count(key) > 0;
    }}
    
    // 转换为Dart Map
    Map<{}, {}> toMap() {{
        Map<{}, {}> result = {{}};
        // 注意：这里需要通过FFI迭代来实现
        // 暂时返回空Map，需要额外的FFI支持来实现迭代
        return result;
    }}
"#, 
        class.type_str, 
        key_dart_type, value_dart_type,
        class.type_str,
        value_dart_type,
        key_dart_type, value_dart_type,
        key_dart_type, value_dart_type
    )
}

/// 为StdSet类生成便利方法
fn generate_stdset_convenience_methods(class: &Class) -> String {
    let Some(value_type) = class.value_type.as_ref() else { return String::new(); };
    let value_dart_type = get_str_dart_fun_type(value_type);
    
    format!(r#"
    // 便利构造函数 - 从Dart Set创建
    {}.fromSet(Set<{}> set) {{
        _nativePtr = ffi_{}_Constructor();
        nativeLifecycleLink();
        for (var value in set) {{
            insert(value);
        }}
    }}
    
    // length属性
    int get length => size();
    
    // contains方法
    bool contains(dynamic value) {{
        return count(value) > 0;
    }}
    
    // 转换为Dart Set
    Set<{}> toSet() {{
        Set<{}> result = {{}};
        // 注意：这里需要通过FFI迭代来实现
        // 暂时返回空Set，需要额外的FFI支持来实现迭代
        return result;
    }}
"#, 
        class.type_str, 
        value_dart_type,
        class.type_str,
        value_dart_type,
        value_dart_type
    )
}

/// 为StdUnorderedSet类生成便利方法
fn generate_stdunorderedset_convenience_methods(class: &Class) -> String {
    let Some(value_type) = class.value_type.as_ref() else { return String::new(); };
    let value_dart_type = get_str_dart_fun_type(value_type);
    
    format!(r#"
    // 便利构造函数 - 从Dart Set创建
    {}.fromSet(Set<{}> set) {{
        _nativePtr = ffi_{}_Constructor();
        nativeLifecycleLink();
        for (var value in set) {{
            insert(value);
        }}
    }}
    
    // length属性
    int get length => size();
    
    // contains方法
    bool contains(dynamic value) {{
        return count(value) > 0;
    }}
    
    // 转换为Dart Set
    Set<{}> toSet() {{
        Set<{}> result = {{}};
        // 注意：这里需要通过FFI迭代来实现
        // 暂时返回空Set，需要额外的FFI支持来实现迭代
        return result;
    }}
"#, 
        class.type_str, 
        value_dart_type,
        class.type_str,
        value_dart_type,
        value_dart_type
    )
}

/// 生成 Dart enum 代码
fn gen_dart_enum(enum_def: &Enum) -> String {
    let comment = enum_def.comment_str.as_ref().map(|c| format!("{}\n", c)).unwrap_or_default();

    if enum_def.is_scoped {
        // enum class → 生成 Dart enum（强类型）
        gen_dart_scoped_enum(enum_def, &comment)
    } else {
        // 普通 enum → 生成 class + static const（兼容）
        gen_dart_unscoped_enum(enum_def, &comment)
    }
}

/// 为 enum class 生成 Dart enum
fn gen_dart_scoped_enum(enum_def: &Enum, comment: &str) -> String {
    let mut enum_values = Vec::new();

    for (name, value) in &enum_def.values {
        // 转换为 lowerCamelCase（Dart 枚举值规范）
        let dart_name = to_lower_camel_case(name);
        enum_values.push(format!("  {}({})", dart_name, value));
    }

    format!(
r#"{}enum {} {{
{};

  final int value;
  const {}(this.value);

  static {}? fromValue(int value) {{
    try {{
      return {}.values.firstWhere(
        (e) => e.value == value,
      );
    }} catch (_) {{
      return null;
    }}
  }}
}}

"#,
        comment,
        enum_def.name,
        enum_values.join(",\n"),
        enum_def.name,
        enum_def.name,
        enum_def.name
    )
}

/// 为普通 enum 生成 Dart class
fn gen_dart_unscoped_enum(enum_def: &Enum, comment: &str) -> String {
    let mut const_values = Vec::new();

    for (name, value) in &enum_def.values {
        // 普通 enum 的值名称保持 UPPER_CASE
        const_values.push(format!("  static const int {} = {};", name, value));
    }

    format!(
r#"{}// 注意：这是普通 enum，建议在 C++ 中改为 enum class
class {} {{
{}
}}

"#,
        comment,
        enum_def.name,
        const_values.join("\n")
    )
}

/// 将字符串转换为 lowerCamelCase
fn to_lower_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split('_').collect();
    if parts.is_empty() {
        return s.to_lowercase();
    }

    let mut result = parts[0].to_lowercase();
    for part in &parts[1..] {
        if !part.is_empty() {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_uppercase().next().unwrap());
                result.push_str(&chars.as_str().to_lowercase());
            }
        }
    }
    result
}
