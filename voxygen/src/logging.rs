use fern::colors::{Color, ColoredLevelConfig};

pub fn init(term_log_level: log::LevelFilter, file_log_level: log::LevelFilter) {
    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Cyan)
        .debug(Color::Green)
        .trace(Color::BrightBlack);

    let base = fern::Dispatch::new()
        .level_for("dot_vox::parser", log::LevelFilter::Warn)
        .level_for("gfx_device_gl::factory", log::LevelFilter::Warn)
        .level_for("veloren_voxygen::discord", log::LevelFilter::Warn)
        .level_for("uvth", log::LevelFilter::Warn)
        .level_for("tiny_http", log::LevelFilter::Warn);

    let time = chrono::offset::Utc::now();

    let file_cfg = fern::Dispatch::new()
        .level(file_log_level)
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}:{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record
                    .line()
                    .map(|x| x.to_string())
                    .unwrap_or("X".to_string()),
                record.level(),
                message
            ))
        })
        .chain(
            fern::log_file(&format!("voxygen-{}.log", time.format("%Y-%m-%d-%H")))
                .expect("Failed to create log file!"),
        );

    let stdout_cfg = fern::Dispatch::new()
        .level(term_log_level)
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] {}",
                colors.color(record.level()),
                message
            ))
        })
        .chain(std::io::stdout());

    base.chain(file_cfg)
        .chain(stdout_cfg)
        .apply()
        .expect("Failed to setup logging!");
}
