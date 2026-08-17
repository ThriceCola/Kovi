use std::sync::Arc;

use kovi::driver::DriverEvent;
use kovi::{Bot, PluginBuilder};

use crate::harness::{
    MockDriver, RunOutcome, conf, observe_run, spawn_bot, status_file_guard, wait_ready,
};

/// 事件流 yield Err 时，kovi 会发 FromDrive 并让 Bot::run 返回。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_stream_error_exits() {
    let _guard = status_file_guard();
    let (driver, tx, ready) = MockDriver::new();
    let ready_wait = ready.notified();
    let handle = spawn_bot(driver);
    wait_ready(ready_wait).await;

    tx.send(Err("mock websocket error".into()))
        .await
        .expect("send error event");

    let outcome = observe_run(handle).await;
    assert_eq!(
        outcome,
        RunOutcome::ExitedFromDrive,
        "event stream Err should make Bot::run exit"
    );
}

/// 事件流 yield DriverEvent::Exit 时，Bot::run 返回。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_stream_exit_event_exits() {
    let _guard = status_file_guard();
    let (driver, tx, ready) = MockDriver::new();
    let ready_wait = ready.notified();
    let handle = spawn_bot(driver);
    wait_ready(ready_wait).await;

    tx.send(Ok(DriverEvent::Exit))
        .await
        .expect("send exit event");

    let outcome = observe_run(handle).await;
    assert_eq!(
        outcome,
        RunOutcome::ExitedFromDrive,
        "DriverEvent::Exit should make Bot::run exit"
    );
}

/// 事件流静默结束（服务端正常关连接后 Stream 返回 None）时，
/// Bot::run 以 FromDrive 退出。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_stream_silent_end_exits() {
    let _guard = status_file_guard();
    let (driver, tx, ready) = MockDriver::new();
    let ready_wait = ready.notified();
    let handle = spawn_bot(driver);
    wait_ready(ready_wait).await;

    drop(tx);

    let outcome = observe_run(handle).await;
    assert_eq!(
        outcome,
        RunOutcome::ExitedFromDrive,
        "silent event stream end should make Bot::run exit with FromDrive"
    );
}

/// 有插件在跑时，事件流静默结束同样要退出。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_stream_silent_end_exits_even_with_plugin() {
    let _guard = status_file_guard();
    let (driver, tx, ready) = MockDriver::new();
    let mut bot = Bot::build(conf(), driver);
    bot.mount_plugin(kovi::plugin::Plugin::new(
        "probe",
        "0.0.0",
        Arc::new(|| {
            Box::pin(async {
                let _bot = PluginBuilder::get_runtime_bot();
            })
        }),
    ));
    let ready_wait = ready.notified();
    let handle = tokio::spawn(bot.run());
    wait_ready(ready_wait).await;
    drop(tx);

    let outcome = observe_run(handle).await;
    assert_eq!(outcome, RunOutcome::ExitedFromDrive);
}

/// 先收到正常事件，再让流结束，仍然退出而不是挂死。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn normal_events_then_silent_end_exits() {
    let _guard = status_file_guard();
    let (driver, tx, ready) = MockDriver::new();
    let ready_wait = ready.notified();
    let handle = spawn_bot(driver);
    wait_ready(ready_wait).await;

    tx.send(Ok(DriverEvent::Normal(kovi::serde_json::json!({
        "post_type": "meta_event"
    }))))
    .await
    .expect("send normal event");
    tx.send(Ok(DriverEvent::Normal(kovi::serde_json::json!({
        "post_type": "message"
    }))))
    .await
    .expect("send second event");
    drop(tx);

    let outcome = observe_run(handle).await;
    assert_eq!(outcome, RunOutcome::ExitedFromDrive);
}

/// 正常事件之后再 yield Err，仍然退出。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn normal_event_then_error_exits() {
    let _guard = status_file_guard();
    let (driver, tx, ready) = MockDriver::new();
    let ready_wait = ready.notified();
    let handle = spawn_bot(driver);
    wait_ready(ready_wait).await;

    tx.send(Ok(DriverEvent::Normal(kovi::serde_json::json!({
        "post_type": "meta_event"
    }))))
    .await
    .expect("send normal event");
    tx.send(Err("later websocket error".into()))
        .await
        .expect("send error");

    let outcome = observe_run(handle).await;
    assert_eq!(outcome, RunOutcome::ExitedFromDrive);
}
