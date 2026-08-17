use kovi::Bot;

use crate::harness::{
    FailingChannelDriver, MockDriver, RunOutcome, conf, observe_run, spawn_bot, status_file_guard,
    wait_ready,
};

/// event_channel() 一开始就失败时，Bot::run 同样以 FromDrive 退出。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_channel_open_failure_exits() {
    let _guard = status_file_guard();
    let handle = tokio::spawn(Bot::build(conf(), FailingChannelDriver).run());
    let outcome = observe_run(handle).await;
    assert_eq!(
        outcome,
        RunOutcome::ExitedFromDrive,
        "failed event_channel should make Bot::run exit with FromDrive"
    );
}

/// 流一直开着、既不结束也不报错时，Bot::run 保持运行。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_event_stream_keeps_running() {
    let (driver, tx, ready) = MockDriver::new();
    let ready_wait = ready.notified();
    let handle = spawn_bot(driver);
    wait_ready(ready_wait).await;

    let outcome = observe_run(handle).await;
    drop(tx);
    assert_eq!(
        outcome,
        RunOutcome::Hung,
        "an open event stream must not exit by itself"
    );
}
