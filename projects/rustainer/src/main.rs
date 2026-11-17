use nix::mount::{MsFlags, mount};
use nix::sched::{CloneFlags, unshare};
use nix::unistd::{self, sethostname};
use std::os::unix::process::CommandExt;
use std::process::Command;
// Call: Command::new("/bin/sh").status()

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    println!("Current system hostname {:?}", unistd::gethostname()?);
    unshare(CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNS)?;

    sethostname("rustainer-box")?;
    println!("Created hostname {:?}", unistd::gethostname()?);

    let mut cmd = Command::new("/bin/sh");

    unsafe {
        cmd.pre_exec(|| {
            mount(
                None::<&str>,
                "/",
                None::<&str>,
                MsFlags::MS_REC | MsFlags::MS_PRIVATE,
                None::<&str>,
            )?;

            mount(
                Some("proc"),
                "/proc",
                Some("proc"),
                MsFlags::empty(),
                None::<&str>,
            )?;

            Ok(())
        });
    }

    cmd.status()?;

    Ok(())
}
