use anyhow::Result;

fn main() -> Result<()> {
    bleepinit::logger::init_logging("info")?;
    if std::process::id() == 1 {
        bleepinit::pid1::setup_pid1()
    } else {
        println!("bleepinit: not running as PID 1, use 'bleepinit --help' for service manager CLI");
        Ok(())
    }
}
