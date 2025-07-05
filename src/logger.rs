#[cfg(feature = "logger")]
pub fn set_logger() {
    use chrono::Local;
    use log::Level;
    use std::io::Write;

    macro_rules! _format {
        ($level:literal, $timestamp:ident, $record:ident, $($color:ident),+ $(,)?) => {{
            let color = dialoguer::console::style($level)$(.$color())+;
            format!("[{}] [{}]: {}", color, $timestamp, $record.args())
        }};
      }

    let init = env_logger::Builder::from_default_env()
        .format(|buf, record| {
            let t = Local::now().format("%m-%d %H:%M:%S");

            match record.level() {
                Level::Info => {
                    writeln!(buf, "[{t}] {}", record.args())
                }
                Level::Debug => {
                    writeln!(buf, "{}", _format!("Debug", t, record, yellow, italic))
                }
                Level::Warn => {
                    writeln!(buf, "{}", _format!("Warn", t, record, yellow))
                }
                Level::Error => {
                    writeln!(buf, "{}", _format!("Error", t, record, red))
                }
                Level::Trace => {
                    writeln!(buf, "{}", _format!("Trace", t, record, magenta))
                }
            }
        })
        .try_init();

    if let Err(e) = init {
        println!(
            "Kovi init env_logger failed: {e}. Very likely you've already started a logger"
        );
    }
}

pub fn try_set_logger() {
    #[cfg(feature = "logger")]
    set_logger();
}

#[cfg(feature = "logger")]
#[test]
fn test_logger() {
    unsafe {
        std::env::set_var("RUST_LOG", "trace");
    }

    // Initialize the logger
    try_set_logger();

    // Test different log levels
    log::info!("This is an info message - should appear without color");
    log::debug!("This is a debug message - should appear in yellow");
    log::warn!("This is a warning message - should appear in yellow");
    log::error!("This is an error message - should appear in red");
    log::trace!("This is a trace message - should appear in red");
}
