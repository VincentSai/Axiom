use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use axiom_ffi_gen::{generate_c_abi_rust_glue, generate_c_header};
use axiom_parser::parse_module;
use axiom_transpiler::transpile_to_rust;

const DECIMAL_MODULE_SOURCE: &str = r#"
    fn decimal_identity(x: Decimal) -> Decimal {
        x
    }

    fn decimal_add(a: Decimal, b: Decimal) -> Decimal {
        a + b
    }

    fn decimal_double(x: Decimal) -> Decimal {
        x * 2.0d
    }
"#;

#[test]
fn test_decimal_abi_c_header_and_rust_glue_match_micros_int64() {
    let module = parse_module(DECIMAL_MODULE_SOURCE).expect("decimal source should parse");

    // 1. 验证 C 头文件生成
    let header = generate_c_header(&module, "DECIMAL_ABI_TEST_H").expect("header should generate");
    assert!(
        header.contains("#define AXIOM_DECIMAL_SCALE 1000000LL"),
        "C header MUST define AXIOM_DECIMAL_SCALE macro! Got:\n{header}"
    );
    assert!(
        header.contains("typedef int64_t axiom_decimal_t;"),
        "C header MUST typedef axiom_decimal_t as int64_t! Got:\n{header}"
    );
    assert!(
        header.contains("int64_t decimal_identity(int64_t x);"),
        "C header MUST declare decimal_identity with int64_t parameter and return! Got:\n{header}"
    );
    assert!(
        header.contains("int64_t decimal_add(int64_t a, int64_t b);"),
        "C header MUST declare decimal_add with int64_t parameters and return! Got:\n{header}"
    );
    assert!(
        !header.contains("double decimal_identity"),
        "C header MUST NOT declare Decimal as double float! Got:\n{header}"
    );

    // 2. 验证 Rust C-ABI 胶水代码生成
    let glue = generate_c_abi_rust_glue(&module).expect("glue should generate");
    assert!(
        glue.contains("pub extern \"C\" fn axiom_decimal_identity(x: i64) -> i64"),
        "Rust FFI glue MUST expose axiom_decimal_identity with i64 ABI! Got:\n{glue}"
    );
    assert!(
        glue.contains("pub extern \"C\" fn axiom_decimal_add(a: i64, b: i64) -> i64"),
        "Rust FFI glue MUST expose axiom_decimal_add with i64 ABI! Got:\n{glue}"
    );
    assert!(
        glue.contains("pub extern \"C\" fn axiom_decimal_double(x: i64) -> i64"),
        "Rust FFI glue MUST expose axiom_decimal_double with i64 ABI! Got:\n{glue}"
    );
    assert!(
        !glue.contains("fn decimal_identity(x: f64) -> f64"),
        "Rust FFI glue MUST NOT declare Decimal as f64 float! Got:\n{glue}"
    );
}

#[test]
fn test_decimal_abi_end_to_end_linking_and_invocation() {
    let module = parse_module(DECIMAL_MODULE_SOURCE).expect("decimal source should parse");

    // 生成 Rust 原生实现
    let impl_code = transpile_to_rust(&module).expect("transpiler should succeed");
    // 生成 C-ABI 胶水代码
    let glue_code = generate_c_abi_rust_glue(&module).expect("glue should generate");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock valid")
        .as_nanos();
    let temp_dir =
        std::env::temp_dir().join(format!("axiom-decimal-link-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("temporary directory should be created");

    // 1. 构造供 cdylib 导出的完整库源码
    let lib_source = format!(
        r#"
// 原生实现模块
mod transpiled {{
{impl_code}
}}

// 宿主程序提供对应 extern "Rust" 符号
#[no_mangle]
pub fn decimal_identity(x: i64) -> i64 {{
    transpiled::decimal_identity(x)
}}
#[no_mangle]
pub fn decimal_add(a: i64, b: i64) -> i64 {{
    transpiled::decimal_add(a, b)
}}
#[no_mangle]
pub fn decimal_double(x: i64) -> i64 {{
    transpiled::decimal_double(x)
}}

// FFI 胶水代码 (放于子模块中以消除与实现的同名声明冲突，并导出 #[no_mangle] pub extern "C" fn axiom_*)
mod ffi {{
{glue_code}
}}
pub use ffi::*;
"#
    );

    let lib_path = temp_dir.join("lib_export.rs");
    fs::write(&lib_path, lib_source).expect("lib source should be written");

    // 2. 编译为真正的 C-ABI 动态链接库 (cdylib)
    let compile_cdylib = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=cdylib")
        .arg("--crate-name=axiom_decimal")
        .arg("-A")
        .arg("warnings")
        .arg(&lib_path)
        .arg("--out-dir")
        .arg(&temp_dir)
        .output()
        .expect("rustc should compile cdylib");

    assert!(
        compile_cdylib.status.success(),
        "Failed to compile cdylib:\n{}",
        String::from_utf8_lossy(&compile_cdylib.stderr)
    );

    // 兼容 Windows MSVC 导入库命名 (axiom_decimal.dll.lib -> axiom_decimal.lib)
    let dll_lib = temp_dir.join("axiom_decimal.dll.lib");
    if dll_lib.exists() {
        let _ = fs::copy(&dll_lib, temp_dir.join("axiom_decimal.lib"));
    }

    // 3. 编写跨语言调用端 Harness (模拟 C / C# / FFI 调用方按 64位整数微元约定传参并链接动态库)
    let harness_source = r#"
extern "C" {
    fn axiom_decimal_identity(x: i64) -> i64;
    fn axiom_decimal_add(a: i64, b: i64) -> i64;
    fn axiom_decimal_double(x: i64) -> i64;
}

fn main() {
    // 3.1 测试正小数 1.5d (1_500_000 micros)
    let val_1_5 = 1_500_000_i64;
    let ret = unsafe { axiom_decimal_identity(val_1_5) };
    assert_eq!(ret, 1_500_000, "identity(1.5d) must return 1500000 micros, got {ret}");

    // 3.2 测试负小数 -2.75d (-2_750_000 micros)
    let val_neg_2_75 = -2_750_000_i64;
    let ret_neg = unsafe { axiom_decimal_identity(val_neg_2_75) };
    assert_eq!(ret_neg, -2_750_000, "identity(-2.75d) must return -2750000 micros, got {ret_neg}");

    // 3.3 测试加法 1.5d + (-2.75d) = -1.25d (-1_250_000 micros)
    let sum = unsafe { axiom_decimal_add(val_1_5, val_neg_2_75) };
    assert_eq!(sum, -1_250_000, "1.5d + (-2.75d) must return -1250000 micros, got {sum}");

    // 3.4 测试乘法翻倍 1.5d * 2.0d = 3.0d (3_000_000 micros)
    let dbl = unsafe { axiom_decimal_double(val_1_5) };
    assert_eq!(dbl, 3_000_000, "1.5d * 2.0d must return 3000000 micros, got {dbl}");

    println!("DECIMAL_ABI_LINK_SUCCESS");
}
"#;

    let harness_path = temp_dir.join("harness.rs");
    let exe_path = temp_dir.join("harness.exe");
    fs::write(&harness_path, harness_source).expect("harness source should be written");

    // 4. 编译 harness 并链接生成的动态库
    let compile_harness = Command::new("rustc")
        .arg("--edition=2021")
        .arg("-A")
        .arg("warnings")
        .arg("-L")
        .arg(&temp_dir)
        .arg("-l")
        .arg("axiom_decimal")
        .arg(&harness_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("rustc should link harness");

    assert!(
        compile_harness.status.success(),
        "Failed to link decimal ABI harness with cdylib:\n{}",
        String::from_utf8_lossy(&compile_harness.stderr)
    );

    // 5. 实际运行可执行二进制并验证跨 ABI 调用结果 (将 temp_dir 加入 PATH 以加载 dll)
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{};{}", temp_dir.display(), current_path);

    let run_output = Command::new(&exe_path)
        .env("PATH", new_path)
        .output()
        .expect("compiled harness binary should run");

    let stdout_str = String::from_utf8_lossy(&run_output.stdout);
    let stderr_str = String::from_utf8_lossy(&run_output.stderr);

    // 清理临时文件
    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run_output.status.success(),
        "Decimal ABI harness exited with failure (code: {:?}):\nstdout: {}\nstderr: {}",
        run_output.status.code(),
        stdout_str,
        stderr_str
    );
    assert!(
        stdout_str.contains("DECIMAL_ABI_LINK_SUCCESS"),
        "Harness did not print expected success marker! Got:\n{stdout_str}"
    );
}
