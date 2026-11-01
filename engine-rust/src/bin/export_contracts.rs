//! 导出引擎线合同 JSON Schema。
//!
//! 用法：`export_contracts <输出目录>` 写出；`export_contracts --check <目录>`
//! 逐字节比对不写盘（门禁用）。Schema 由 serde 类型经 schemars 生成，
//! 见 `yixian_engine::contracts`。

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let str_args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (check, dir): (bool, &str) = match str_args.as_slice() {
        [dir] => (false, *dir),
        ["--check", dir] => (true, *dir),
        _ => {
            eprintln!("usage: export_contracts [--check] <dir>");
            std::process::exit(2);
        }
    };
    let dir = std::path::Path::new(dir);
    let result = if check {
        yixian_engine::contracts::check(dir).map(|()| "wire contract schemas are current".to_string())
    } else {
        yixian_engine::contracts::write(dir).map(|()| format!("wrote wire contract schemas to {}", dir.display()))
    };
    match result {
        Ok(message) => println!("{message}"),
        Err(message) => {
            eprintln!("Error: {message}");
            std::process::exit(1);
        }
    }
}
