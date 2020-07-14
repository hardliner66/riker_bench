pub fn init() {
    let base_config = fern::Dispatch::new();

    let stdout_config = fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%H:%M:%S]"),
                record.target(),
                message
            ))
        })
        // Add blanket level filter -
        .level(log::LevelFilter::Trace)
        // Output to stdout, files, and other Dispatch configurations
        .chain(std::io::stdout());

    let log_to_file = false && !std::env::args().any(|arg| arg.to_lowercase() == "--no-log");

    if log_to_file {
        std::fs::create_dir_all("logs").expect("Could not create log directory!");

        let log_file_name = chrono::Local::now().format("logs/output-%Y_%m_%d-%H_%M_%S.log");

        let file_config = fern::Dispatch::new()
            // Perform allocation-free log formatting
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{}] {}[{}] {}",
                    record.level(),
                    chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(),
                    message
                ))
            })
            .level(log::LevelFilter::Trace)
            .chain(
                fern::log_file(log_file_name.to_string())
                    .expect(format!("Could not open log file: \"{}\"", log_file_name).as_str()),
            );

        base_config
            .chain(file_config)
            .chain(stdout_config)
            .apply()
            .expect("Failed to set up logging!")
    } else {
        base_config
            .chain(stdout_config)
            .apply()
            .expect("Failed to set up logging!");
    }
}
