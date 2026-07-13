// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match shelfy_lib::updater::run_helper(&args) {
        Ok(true) => return,
        Err(error) => {
            eprintln!("Update helper failed: {error}");
            return;
        }
        Ok(false) => {}
    }
    if shelfy_lib::try_run_cli() {
        return;
    }
    shelfy_lib::run()
}
