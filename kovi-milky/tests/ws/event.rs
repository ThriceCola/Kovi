use std::time::Duration;

use futures_util::StreamExt;
use kovi::Bot;
use kovi::driver::{Driver, DriverEvent};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

use crate::harness::{
    CONNECT_TIMEOUT, Disconnect, MockMilky, RunOutcome, StreamOutcome, bot_after_event_disconnect,
    drain_until_break, event_stream_after_disconnect, observe_run, status_file_guard,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_ws_normal_close_emits_exit() {
    let outcome = event_stream_after_disconnect(Disconnect::Close).await;
    assert_eq!(
        outcome,
        StreamOutcome::ExitEvent,
        "event WS Close should yield DriverEvent::Exit"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_ws_normal_close_bot_exits() {
    let outcome = bot_after_event_disconnect(Disconnect::Close).await;
    assert_eq!(
        outcome,
        RunOutcome::ExitedFromDrive,
        "normal server close should make Bot::run exit with FromDrive"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_ws_tcp_drop_yields_err() {
    let outcome = event_stream_after_disconnect(Disconnect::Drop).await;
    assert_eq!(
        outcome,
        StreamOutcome::StreamErr,
        "abnormal event WS drop yields Err"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn event_ws_tcp_drop_bot_exits() {
    let outcome = bot_after_event_disconnect(Disconnect::Drop).await;
    assert_eq!(
        outcome,
        RunOutcome::ExitedFromDrive,
        "abnormal event WS drop should make Bot::run exit"
    );
}

/// 事件非法 JSON：只警告，不结束流；Bot 也不退出。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_json_event_is_ignored() {
    let _guard = status_file_guard();
    let server = MockMilky::start().await;
    let mut stream = server
        .driver()
        .event_channel()
        .await
        .expect("event_channel");
    server.wait_event().await;
    server.send_event(Message::text("{not-json")).await;
    let ignored = timeout(Duration::from_millis(200), stream.next()).await;
    assert!(
        ignored.is_err(),
        "invalid JSON must not end the event stream"
    );
    server
        .send_event(Message::text(r#"{"event_type":"bot_offline"}"#))
        .await;
    match timeout(CONNECT_TIMEOUT, stream.next()).await {
        Ok(Some(Ok(DriverEvent::Normal(_)))) => {}
        Ok(Some(Ok(DriverEvent::Exit))) => panic!("got Exit instead of Normal event"),
        Ok(Some(Err(_))) => panic!("got stream Err instead of Normal event"),
        Ok(None) => panic!("stream ended instead of delivering Normal event"),
        Err(_) => panic!("timed out waiting for valid JSON event"),
    }
    server.disconnect_event(Disconnect::Close).await;
    assert_eq!(
        drain_until_break(&mut stream).await,
        StreamOutcome::ExitEvent
    );

    let server = MockMilky::start().await;
    let handle = tokio::spawn(Bot::build(crate::harness::conf(), server.driver()).run());
    server.wait_event().await;
    server.send_event(Message::text("{not-json")).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !handle.is_finished(),
        "invalid JSON must not make Bot::run exit"
    );
    server.disconnect_event(Disconnect::Close).await;
    assert_eq!(observe_run(handle).await, RunOutcome::ExitedFromDrive);
}

/// 二进制帧只警告，不结束流。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn binary_event_frame_is_ignored() {
    let server = MockMilky::start().await;
    let mut stream = server
        .driver()
        .event_channel()
        .await
        .expect("event_channel");
    server.wait_event().await;
    server.send_event(Message::binary(vec![9u8, 8, 7])).await;
    let ignored = timeout(Duration::from_millis(200), stream.next()).await;
    assert!(
        ignored.is_err(),
        "binary frame must not end the event stream"
    );
    server.disconnect_event(Disconnect::Close).await;
    assert_eq!(
        drain_until_break(&mut stream).await,
        StreamOutcome::ExitEvent
    );
}

/// Ping 被吞掉，随后 Close 仍 Exit。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ping_keeps_stream_alive_then_close_exits() {
    let server = MockMilky::start().await;
    let mut stream = server
        .driver()
        .event_channel()
        .await
        .expect("event_channel");
    server.wait_event().await;
    server.send_event(Message::Ping(Default::default())).await;
    let ping_wait = timeout(Duration::from_millis(200), stream.next()).await;
    assert!(
        ping_wait.is_err(),
        "ping should be swallowed, not emitted as an event"
    );
    server.disconnect_event(Disconnect::Close).await;
    assert_eq!(
        drain_until_break(&mut stream).await,
        StreamOutcome::ExitEvent
    );
}

/// 先推合法 JSON 事件再 Close，仍然 Exit。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_event_then_close_emits_exit() {
    let server = MockMilky::start().await;
    let mut stream = server
        .driver()
        .event_channel()
        .await
        .expect("event_channel");
    server.wait_event().await;
    server
        .send_event(Message::text(r#"{"event_type":"bot_offline"}"#))
        .await;
    server.disconnect_event(Disconnect::Close).await;
    assert_eq!(
        drain_until_break(&mut stream).await,
        StreamOutcome::ExitEvent
    );
}
