use std::sync::Arc;
use std::time::Duration;

use kovi::bot::SendApi;
use kovi::driver::Driver;
use kovi::{Bot, PluginBuilder};
use serde_json::json;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use crate::harness::{
    CONNECT_TIMEOUT, Disconnect, MockOneBot, RunOutcome, conf, observe_run, status_file_guard,
};

/// API WS 正常关闭：不会再建立第二条连接，后续 API 失败或挂起。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_ws_does_not_reconnect_after_close() {
    let server = MockOneBot::start().await;
    let driver = server.driver();

    driver
        .api_handler(SendApi::new("get_login_info", json!({})))
        .await
        .expect("first api connect")
        .expect("first get_login_info");
    server.wait_api().await;
    assert_eq!(server.api_connects(), 1);

    server.disconnect_api(Disconnect::Close).await;

    let second = timeout(
        Duration::from_secs(2),
        driver.api_handler(SendApi::new("get_login_info", json!({}))),
    )
    .await;

    assert_eq!(
        server.api_connects(),
        1,
        "API WS must not be re-established after disconnect"
    );

    match second {
        Err(_) => {}
        Ok(Err(_)) => {}
        Ok(Ok(Err(_))) => {}
        Ok(Ok(Ok(_))) => panic!("second API call succeeded without reconnecting, unexpected"),
    }
}

/// 通过 Bot 插件发第二次 API：断开后不会重连。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_ws_disconnect_plugin_calls_fail() {
    let _guard = status_file_guard();
    let server = MockOneBot::start().await;
    let driver = server.driver();

    let (req_tx, req_rx) =
        mpsc::channel::<oneshot::Sender<Result<kovi::ApiReturn, kovi::ApiReturn>>>(4);
    let req_rx = Arc::new(tokio::sync::Mutex::new(req_rx));
    let (ready_tx, mut ready_rx) = tokio::sync::watch::channel(false);

    let mut bot = Bot::build(conf(), driver);
    bot.mount_plugin(kovi::plugin::Plugin::new(
        "api-probe",
        "0.0.0",
        Arc::new(move || {
            let req_rx = req_rx.clone();
            let ready_tx = ready_tx.clone();
            Box::pin(async move {
                let bot = PluginBuilder::get_runtime_bot();
                let _ = ready_tx.send(true);
                let mut rx = req_rx.lock().await;
                while let Some(reply) = rx.recv().await {
                    let result = bot.send_api_return("get_login_info", json!({})).await;
                    let _ = reply.send(result);
                }
            })
        }),
    ));

    let handle = tokio::spawn(bot.run());
    server.wait_api().await;
    server.wait_event().await;
    timeout(CONNECT_TIMEOUT, ready_rx.wait_for(|ready| *ready))
        .await
        .expect("plugin ready")
        .expect("ready watch closed");

    let (tx1, rx1) = oneshot::channel();
    req_tx.send(tx1).await.expect("send first probe");
    rx1.await
        .expect("first probe oneshot")
        .expect("first plugin API");
    assert_eq!(server.api_connects(), 1);

    server.disconnect_api(Disconnect::Close).await;

    let (tx2, rx2) = oneshot::channel();
    let plugin_still_alive = req_tx.send(tx2).await.is_ok();
    if plugin_still_alive {
        let second = timeout(Duration::from_secs(2), rx2).await;
        match second {
            Err(_) => {}
            Ok(Err(_)) => {}
            Ok(Ok(Err(_))) => {}
            Ok(Ok(Ok(_))) => panic!("plugin second API succeeded after API WS close"),
        }
    }

    assert_eq!(server.api_connects(), 1, "plugin API must not reconnect");

    handle.abort();
    let _ = handle.await;
}

/// API 正常关闭时，onebot 读任务会往事件通道塞 Exit。观测 Bot 是否因此退出。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_ws_close_bot_outcome() {
    let _guard = status_file_guard();
    let server = MockOneBot::start().await;
    let driver = server.driver();
    let handle = tokio::spawn(Bot::build(conf(), driver).run());
    server.wait_api().await;
    server.wait_event().await;
    server.disconnect_api(Disconnect::Close).await;
    let outcome = observe_run(handle).await;
    assert_eq!(
        outcome,
        RunOutcome::ExitedFromDrive,
        "API WS Close currently injects DriverEvent::Exit, so Bot::run returns"
    );
}

/// API WS 异常 drop（没有 Close 帧）时同样退出，不重连。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_ws_tcp_drop_bot_exits() {
    let _guard = status_file_guard();
    let server = MockOneBot::start().await;
    let handle = tokio::spawn(Bot::build(conf(), server.driver()).run());
    server.wait_api().await;
    server.wait_event().await;
    server.disconnect_api(Disconnect::Drop).await;
    let outcome = observe_run(handle).await;
    assert_eq!(
        outcome,
        RunOutcome::ExitedFromDrive,
        "API TCP drop without Close should make Bot::run exit"
    );
    assert_eq!(server.api_connects(), 1, "API WS must not reconnect");
}

/// API 返回 failed 时调用失败，但不会再开第二条 API 连接。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_failed_status_does_not_reconnect() {
    let server = MockOneBot::start().await;
    let driver = server.driver();
    driver
        .api_handler(SendApi::new("get_login_info", json!({})))
        .await
        .expect("connect")
        .expect("first ok");
    server.wait_api().await;
    server.force_api_fail();

    let second = driver
        .api_handler(SendApi::new("get_login_info", json!({})))
        .await
        .expect("second call transport");
    assert!(
        second.is_err(),
        "failed status should surface as Err(ApiReturn)"
    );
    assert_eq!(server.api_connects(), 1);
}

/// 并发 API 调用都能拿到回包，且只建立一条 API 连接。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_api_calls_share_one_connection() {
    let server = MockOneBot::start().await;
    let driver = Arc::new(server.driver());
    driver
        .api_handler(SendApi::new("get_login_info", json!({})))
        .await
        .expect("init")
        .expect("login");
    server.wait_api().await;

    let mut joins = Vec::new();
    for _ in 0..8 {
        let driver = Arc::clone(&driver);
        joins.push(tokio::spawn(async move {
            driver
                .api_handler(SendApi::new("get_status", json!({})))
                .await
        }));
    }
    for join in joins {
        let result = join.await.expect("join").expect("transport");
        assert!(result.is_ok(), "concurrent API should succeed: {result:?}");
    }
    assert_eq!(server.api_connects(), 1);
}
