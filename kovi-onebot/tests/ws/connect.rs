use kovi::Bot;
use kovi::driver::Driver;
use tokio::time::timeout;

use crate::harness::{
    CONNECT_TIMEOUT, Disconnect, MockOneBot, RunOutcome, StreamOutcome, conf, drain_until_break,
    driver_on_port, observe_run, status_file_guard, unused_port,
};

/// 对端没有服务时，event_channel 建连失败。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_channel_fails_when_server_down() {
    let driver = driver_on_port(unused_port().await);
    let result = timeout(CONNECT_TIMEOUT, driver.event_channel()).await;
    match result {
        Ok(Err(_)) => {}
        Ok(Ok(_)) => panic!("event_channel succeeded against a closed port"),
        Err(_) => panic!("event_channel hung instead of failing"),
    }
}

/// 服务端关闭后 Bot::run 因建连失败而退出。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_exits_when_server_down() {
    let _guard = status_file_guard();
    let driver = driver_on_port(unused_port().await);
    let handle = tokio::spawn(Bot::build(conf(), driver).run());
    let outcome = observe_run(handle).await;
    assert_eq!(outcome, RunOutcome::ExitedFromDrive);
}

/// 带 access_token 的客户端在服务端不校验时仍能连上。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn access_token_header_still_connects() {
    let server = MockOneBot::start().await;
    let driver = server.driver_with_token("secret-token");
    let mut stream = driver.event_channel().await.expect("event_channel");
    server.wait_event().await;
    server.disconnect_event(Disconnect::Close).await;
    assert_eq!(
        drain_until_break(&mut stream).await,
        StreamOutcome::ExitEvent
    );
}

/// all-in-one 单 endpoint：API 与事件共用 `/`，正常 Close 仍退出。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn all_in_one_event_close_bot_exits() {
    let _guard = status_file_guard();
    let server = MockOneBot::start().await;
    let handle = tokio::spawn(Bot::build(conf(), server.driver_all_in_one()).run());
    server.wait_api().await;
    server.wait_event().await;
    assert_eq!(server.api_connects(), 1);
    assert_eq!(server.event_connects(), 1);
    server.disconnect_event(Disconnect::Close).await;
    assert_eq!(observe_run(handle).await, RunOutcome::ExitedFromDrive);
}
