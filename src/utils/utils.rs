use crate::prelude::*;

pub fn set_cwd_to_exe_dir() -> io::Result<()> {
    let exe = env::current_exe()?;

    let dir = exe.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "Failed to get executable parent directory",
        )
    })?;

    env::set_current_dir(dir)
}
