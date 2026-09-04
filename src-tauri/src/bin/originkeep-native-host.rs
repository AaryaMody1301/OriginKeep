fn main() {
    if let Err(error) = originkeep_lib::native_host::run() {
        eprintln!("OriginKeep native host failed: {error}");
        std::process::exit(1);
    }
}
