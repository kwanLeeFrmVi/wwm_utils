use crate::core::{pack_map, unpack_map};
use crate::prelude::*;

mod core;
mod prelude;
mod structs;
mod tests;
mod utils;

// TODO: Use results instead of unwraps and panics
fn main() {
    // Don't change CWD - let caller control where output goes
    // utils::set_cwd_to_exe_dir().unwrap();

    let mut args = env::args();

    if args.len() < 2 {
        eprintln!(
            "Make sure to provide at least one argument, file path to unpack, or directory path to pack!"
        );
    } else {
        // Ignore first argument (program name)
        let _ = args.next();

        for (_, arg) in args.enumerate() {
            let path = Path::new(&arg);

            if path.is_file() {
                unpack_map(path);
            } else if path.is_dir() {
                pack_map(path);
            } else {
                eprintln!("Path `{}` does not exist!", path.display());
            }
        }
    }

    println!("Done!");
}
