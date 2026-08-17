use kovi::Bot;
use kovi::driver::Driver;
use tokio::time::timeout;

use crate::harness::{
    CONNECT_TIMEOUT, Disconnect, HttpReply, MockMilky, RunOutcome, StreamOutcome, conf,
    drain_until_break, driver_on_port, observe_run, status_file_guard, unused_port,
};

/// HTTP API 端口未开时，lifecycle 失败，event_channel 返回 Err。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_channel_fails_when_http_api_down() {
    let driver = driver_on_port(unused_port().await);
    let result = timeout(CONNECT_TIMEOUT, driver.event_channel()).await;
    match result {
        Ok(Err(_)) => {}
        Ok(Ok(_)) => panic!("event_channel succeeded against a closed port"),
        Err(_) => panic!("event_channel hung instead of failing"),
    }
}

/// HTTP API 不可达时 Bot::run 退出。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_exits_when_server_down() {
    let _guard = status_file_guard();
    let handle = tokio::spawn(Bot::build(conf(), driver_on_port(unused_port().await)).run());
    assert_eq!(observe_run(handle).await, RunOutcome::ExitedFromDrive);
}

/// get_login_info 返回 failed → lifecycle 失败。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_channel_fails_when_login_status_failed() {
    let server = MockMilky::start().await;
    server.set_http_reply(HttpReply::FailedStatus).await;
    let result = server.driver().event_channel().await;
    assert!(result.is_err());
}

/// get_login_info 返回非 JSON → lifecycle 失败。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_channel_fails_when_login_body_not_json() {
    let server = MockMilky::start().await;
    server.set_http_reply(HttpReply::InvalidJson).await;
    let result = server.driver().event_channel().await;
    assert!(result.is_err());
}

/// HTTP API 在事件通道关闭后仍可独立调用（Milky API 是 HTTP，不跟 WS 绑死）。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_api_still_works_after_event_close() {
    let server = MockMilky::start().await;
    let driver = server.driver();
    let mut stream = driver.event_channel().await.expect("event_channel");
    server.wait_event().await;
    server.disconnect_event(Disconnect::Close).await;
    assert_eq!(
        drain_until_break(&mut stream).await,
        StreamOutcome::ExitEvent
    );

    let result = driver
        .api_handler(kovi::bot::SendApi::new(
            "get_login_info",
            serde_json::json!({}),
        ))
        .await
        .expect("http api after event close");
    assert!(
        result.is_ok(),
        "milky HTTP API should still work: {result:?}"
    );
}
