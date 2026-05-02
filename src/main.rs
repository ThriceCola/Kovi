use kovi::{build_bot, plugins};

async fn _test_2() -> anyhow::Result<()> {
    let driver_config = kovi_onebot::load_local_conf()?;
    let driver = kovi_onebot::OneBotDriver::new(driver_config);
    let kovi_config = kovi::load_local_conf()?;
    let mut bot = kovi::Bot::build(kovi_config, driver);

    kovi::logger::try_set_logger_use_env();

    let plugin_set = plugins!(
        // test_async,
        test_hi,
        // kovi_plugin_cmd,
    );

    bot.mount_plugin_set(plugin_set);
    bot.set_plugin_startup_use_file_ref();

    bot.run().await;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let driver_config = kovi_onebot::load_local_conf()?;
    let driver = kovi_onebot::OneBotDriver::new(driver_config);
    let bot = build_bot!(driver; test_hi);

    bot.run().await;

    Ok(())
}
