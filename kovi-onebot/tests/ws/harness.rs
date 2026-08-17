use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use kovi::Bot;
use kovi::ExitEvent;
use kovi::config::kovi_conf::KoviConf;
use kovi::driver::{Driver, DriverEvent};
use kovi::event::id::ID;
use kovi_onebot::OneBotDriver;
use kovi_onebot::driver::config::{Host, OneBotDriverConfig, Server};
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, mpsc};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::{WebSocketStream, accept_hdr_async};

pub(crate) const HANG_TIMEOUT: Duration = Duration::from_millis(800);
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunOutcome {
    ExitedFromDrive,
    ExitedFromSignal,
    Hung,
    Panicked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StreamOutcome {
    ExitEvent,
    StreamErr,
    StreamEnded,
    Hung,
}

#[derive(Clone, Copy)]
pub(crate) enum Disconnect {
    Close,
    Drop,
}

enum ConnCmd {
    Close,
    Drop,
    Send(Message),
}

pub(crate) struct MockOneBot {
    port: u16,
    event_cmd: Arc<tokio::sync::Mutex<Option<mpsc::Sender<ConnCmd>>>>,
    api_cmd: Arc<tokio::sync::Mutex<Option<mpsc::Sender<ConnCmd>>>>,
    api_connect_count: Arc<AtomicUsize>,
    event_connect_count: Arc<AtomicUsize>,
    api_force_fail: Arc<AtomicUsize>,
    shutdown: Arc<Notify>,
}

impl MockOneBot {
    pub(crate) async fn start() -> Arc<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock onebot");
        let addr: SocketAddr = listener.local_addr().expect("local addr");

        let server = Arc::new(Self {
            port: addr.port(),
            event_cmd: Arc::new(tokio::sync::Mutex::new(None)),
            api_cmd: Arc::new(tokio::sync::Mutex::new(None)),
            api_connect_count: Arc::new(AtomicUsize::new(0)),
            event_connect_count: Arc::new(AtomicUsize::new(0)),
            api_force_fail: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(Notify::new()),
        });

        let accept_server = server.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = accept_server.shutdown.notified() => break,
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { break; };
                        let accept_server = accept_server.clone();
                        tokio::spawn(async move {
                            accept_server.handle_conn(stream).await;
                        });
                    }
                }
            }
        });

        server
    }

    pub(crate) fn driver(&self) -> OneBotDriver {
        OneBotDriver::new(OneBotDriverConfig {
            server: Server::new(
                Host::IpAddr("127.0.0.1".parse().expect("ip")),
                self.port,
                String::new(),
                false,
                "/".into(),
                false,
            ),
        })
    }

    pub(crate) fn driver_all_in_one(&self) -> OneBotDriver {
        OneBotDriver::new(OneBotDriverConfig {
            server: Server::new(
                Host::IpAddr("127.0.0.1".parse().expect("ip")),
                self.port,
                String::new(),
                false,
                "/".into(),
                true,
            ),
        })
    }

    pub(crate) fn driver_with_token(&self, token: &str) -> OneBotDriver {
        OneBotDriver::new(OneBotDriverConfig {
            server: Server::new(
                Host::IpAddr("127.0.0.1".parse().expect("ip")),
                self.port,
                token.to_string(),
                false,
                "/".into(),
                false,
            ),
        })
    }

    async fn handle_conn(&self, stream: TcpStream) {
        let mut path = String::new();
        let ws = match accept_hdr_async(stream, |req: &Request, res: Response| {
            path = req.uri().path().to_string();
            Ok(res)
        })
        .await
        {
            Ok(ws) => ws,
            Err(_) => return,
        };

        let kind = path.trim_matches('/');
        let kind = kind.rsplit('/').next().unwrap_or(kind);

        match kind {
            "event" => self.serve_event(ws).await,
            "api" => self.serve_api(ws).await,
            "" => {
                if self.api_connect_count.load(Ordering::SeqCst) == 0 {
                    self.serve_api(ws).await;
                } else {
                    self.serve_event(ws).await;
                }
            }
            _ => {}
        }
    }

    async fn serve_event(&self, ws: WebSocketStream<TcpStream>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(8);
        *self.event_cmd.lock().await = Some(cmd_tx);
        self.event_connect_count.fetch_add(1, Ordering::SeqCst);
        run_ws_session(ws, cmd_rx, None).await;
    }

    async fn serve_api(&self, ws: WebSocketStream<TcpStream>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(8);
        *self.api_cmd.lock().await = Some(cmd_tx);
        self.api_connect_count.fetch_add(1, Ordering::SeqCst);
        let fail = Arc::clone(&self.api_force_fail);
        let on_text: OnText = Some(Arc::new(move |text: &str| {
            reply_onebot_api(text, fail.load(Ordering::SeqCst) != 0)
        }));
        run_ws_session(ws, cmd_rx, on_text).await;
    }

    pub(crate) async fn wait_event(&self) {
        timeout(CONNECT_TIMEOUT, async {
            loop {
                if self.event_cmd.lock().await.is_some() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("event ws did not connect");
    }

    pub(crate) async fn wait_api(&self) {
        timeout(CONNECT_TIMEOUT, async {
            loop {
                if self.api_connect_count.load(Ordering::SeqCst) > 0 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("api ws did not connect");
    }

    pub(crate) async fn disconnect_event(&self, how: Disconnect) {
        send_disconnect(self.event_cmd.lock().await.as_ref(), how).await;
    }

    pub(crate) async fn disconnect_api(&self, how: Disconnect) {
        send_disconnect(self.api_cmd.lock().await.as_ref(), how).await;
    }

    pub(crate) async fn send_event(&self, msg: Message) {
        let tx = self.event_cmd.lock().await.as_ref().cloned();
        if let Some(tx) = tx {
            let _ = tx.send(ConnCmd::Send(msg)).await;
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    }

    pub(crate) fn force_api_fail(&self) {
        self.api_force_fail.store(1, Ordering::SeqCst);
    }

    pub(crate) fn api_connects(&self) -> usize {
        self.api_connect_count.load(Ordering::SeqCst)
    }

    pub(crate) fn event_connects(&self) -> usize {
        self.event_connect_count.load(Ordering::SeqCst)
    }
}

impl Drop for MockOneBot {
    fn drop(&mut self) {
        self.shutdown.notify_waiters();
    }
}

type OnText = Option<Arc<dyn Fn(&str) -> Option<String> + Send + Sync>>;

fn reply_onebot_api(text: &str, force_fail: bool) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    let echo = v.get("echo").cloned().unwrap_or(json!(""));
    let action = v.get("action").and_then(|a| a.as_str()).unwrap_or("");
    let data = match action {
        "get_login_info" => json!({ "user_id": 10000, "nickname": "mock-bot" }),
        _ => json!({}),
    };
    let status = if force_fail { "failed" } else { "ok" };
    let retcode = if force_fail { 1400 } else { 0 };
    Some(
        json!({
            "status": status,
            "retcode": retcode,
            "data": data,
            "echo": echo,
        })
        .to_string(),
    )
}

async fn run_ws_session(
    mut ws: WebSocketStream<TcpStream>,
    mut cmd_rx: mpsc::Receiver<ConnCmd>,
    on_text: OnText,
) {
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(ConnCmd::Send(msg)) => {
                        if ws.send(msg).await.is_err() {
                            return;
                        }
                    }
                    other => {
                        apply_cmd(&mut ws, other).await;
                        return;
                    }
                }
            }
            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Some(on_text) = &on_text
                            && let Some(resp) = on_text(text.as_ref())
                        {
                            let _ = ws.send(Message::text(resp)).await;
                        }
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = ws.send(Message::Pong(p)).await;
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                    _ => {}
                }
            }
        }
    }
}

async fn apply_cmd(ws: &mut WebSocketStream<TcpStream>, cmd: Option<ConnCmd>) {
    match cmd {
        Some(ConnCmd::Close) => {
            let _ = ws.close(None).await;
        }
        Some(ConnCmd::Drop) | Some(ConnCmd::Send(_)) | None => {}
    }
}

async fn send_disconnect(tx: Option<&mpsc::Sender<ConnCmd>>, how: Disconnect) {
    let Some(tx) = tx else {
        return;
    };
    let cmd = match how {
        Disconnect::Close => ConnCmd::Close,
        Disconnect::Drop => ConnCmd::Drop,
    };
    let _ = tx.send(cmd).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
}

pub(crate) fn conf() -> KoviConf {
    KoviConf::new(ID::new(1i64), None, false)
}

pub(crate) async fn observe_run(mut handle: tokio::task::JoinHandle<ExitEvent>) -> RunOutcome {
    tokio::select! {
        result = &mut handle => match result {
            Ok(ExitEvent::FromDrive) => RunOutcome::ExitedFromDrive,
            Ok(ExitEvent::FromSignal) => RunOutcome::ExitedFromSignal,
            Err(_) => RunOutcome::Panicked,
        },
        _ = tokio::time::sleep(HANG_TIMEOUT) => {
            handle.abort();
            let _ = handle.await;
            RunOutcome::Hung
        }
    }
}

pub(crate) async fn drain_until_break(
    stream: &mut (impl StreamExt<Item = Result<DriverEvent, kovi::driver::AnyError>> + Unpin),
) -> StreamOutcome {
    loop {
        tokio::select! {
            item = stream.next() => match item {
                Some(Ok(DriverEvent::Exit)) => return StreamOutcome::ExitEvent,
                Some(Ok(DriverEvent::Normal(_))) => continue,
                Some(Err(_)) => return StreamOutcome::StreamErr,
                None => return StreamOutcome::StreamEnded,
            },
            _ = tokio::time::sleep(HANG_TIMEOUT) => return StreamOutcome::Hung,
        }
    }
}

pub(crate) fn status_file_guard() -> StatusFileGuard {
    StatusFileGuard {
        plugin_existed: std::path::Path::new("kovi.plugin.toml").exists(),
        conf_existed: std::path::Path::new("kovi.conf.toml").exists(),
    }
}

pub(crate) struct StatusFileGuard {
    plugin_existed: bool,
    conf_existed: bool,
}

impl Drop for StatusFileGuard {
    fn drop(&mut self) {
        if !self.plugin_existed {
            let _ = std::fs::remove_file("kovi.plugin.toml");
        }
        if !self.conf_existed {
            let _ = std::fs::remove_file("kovi.conf.toml");
        }
    }
}

pub(crate) async fn bot_after_event_disconnect(how: Disconnect) -> RunOutcome {
    let _guard = status_file_guard();
    let server = MockOneBot::start().await;
    let driver = server.driver();
    let handle = tokio::spawn(Bot::build(conf(), driver).run());
    server.wait_api().await;
    server.wait_event().await;
    server.disconnect_event(how).await;
    observe_run(handle).await
}

pub(crate) async fn event_stream_after_disconnect(how: Disconnect) -> StreamOutcome {
    let server = MockOneBot::start().await;
    let driver = server.driver();
    let mut stream = driver.event_channel().await.expect("event_channel");
    server.wait_event().await;
    server.disconnect_event(how).await;
    drain_until_break(&mut stream).await
}

pub(crate) async fn unused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}

pub(crate) fn driver_on_port(port: u16) -> OneBotDriver {
    OneBotDriver::new(OneBotDriverConfig {
        server: Server::new(
            Host::IpAddr("127.0.0.1".parse().expect("ip")),
            port,
            String::new(),
            false,
            "/".into(),
            false,
        ),
    })
}
