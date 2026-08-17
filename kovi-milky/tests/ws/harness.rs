use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use kovi::config::kovi_conf::KoviConf;
use kovi::driver::{Driver, DriverEvent};
use kovi::event::id::ID;
use kovi::{Bot, ExitEvent};
use kovi_milky::MilkyDriver;
use kovi_milky::driver::config::{Host, MilkyDriverConfig, Server};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

#[derive(Clone)]
pub(crate) enum HttpReply {
    LoginOk,
    FailedStatus,
    InvalidJson,
}

pub(crate) struct MockMilky {
    port: u16,
    event_cmd: Arc<tokio::sync::Mutex<Option<mpsc::Sender<ConnCmd>>>>,
    http_reply: Arc<tokio::sync::Mutex<HttpReply>>,
    shutdown: Arc<Notify>,
}

impl MockMilky {
    pub(crate) async fn start() -> Arc<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock milky");
        let addr: SocketAddr = listener.local_addr().expect("local addr");

        let server = Arc::new(Self {
            port: addr.port(),
            event_cmd: Arc::new(tokio::sync::Mutex::new(None)),
            http_reply: Arc::new(tokio::sync::Mutex::new(HttpReply::LoginOk)),
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

    pub(crate) fn driver(&self) -> MilkyDriver {
        MilkyDriver::new(MilkyDriverConfig {
            server: Server::new(
                Host::IpAddr("127.0.0.1".parse().expect("ip")),
                self.port,
                String::new(),
                false,
                "/".into(),
            ),
        })
    }

    async fn handle_conn(&self, stream: TcpStream) {
        let mut peek = [0u8; 2048];
        let n = match stream.peek(&mut peek).await {
            Ok(n) => n,
            Err(_) => return,
        };
        let head = String::from_utf8_lossy(&peek[..n]).to_ascii_lowercase();
        if head.contains("upgrade: websocket") {
            self.handle_ws(stream).await;
        } else {
            let reply = self.http_reply.lock().await.clone();
            let _ = handle_http(stream, reply).await;
        }
    }

    async fn handle_ws(&self, stream: TcpStream) {
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
        if kind != "event" {
            return;
        }

        let (cmd_tx, cmd_rx) = mpsc::channel(4);
        *self.event_cmd.lock().await = Some(cmd_tx);
        run_event_ws(ws, cmd_rx).await;
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

    pub(crate) async fn disconnect_event(&self, how: Disconnect) {
        let guard = self.event_cmd.lock().await;
        let Some(tx) = guard.as_ref() else {
            return;
        };
        let cmd = match how {
            Disconnect::Close => ConnCmd::Close,
            Disconnect::Drop => ConnCmd::Drop,
        };
        let _ = tx.send(cmd).await;
        drop(guard);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    pub(crate) async fn send_event(&self, msg: Message) {
        let tx = self.event_cmd.lock().await.as_ref().cloned();
        if let Some(tx) = tx {
            let _ = tx.send(ConnCmd::Send(msg)).await;
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    }

    pub(crate) async fn set_http_reply(&self, reply: HttpReply) {
        *self.http_reply.lock().await = reply;
    }
}

impl Drop for MockMilky {
    fn drop(&mut self) {
        self.shutdown.notify_waiters();
    }
}

async fn handle_http(mut stream: TcpStream, reply: HttpReply) -> std::io::Result<()> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(header_end) = find_double_crlf(&buf) {
            let header = String::from_utf8_lossy(&buf[..header_end]);
            let content_length = parse_content_length(&header);
            while buf.len() < header_end + 4 + content_length {
                let n = stream.read(&mut tmp).await?;
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
            }
            break;
        }
        if buf.len() > 64 * 1024 {
            break;
        }
    }

    let body = match reply {
        HttpReply::LoginOk => serde_json::json!({
            "status": "ok",
            "retcode": 0,
            "message": serde_json::Value::Null,
            "data": { "uin": 10000, "nickname": "mock-bot" },
        })
        .to_string(),
        HttpReply::FailedStatus => serde_json::json!({
            "status": "failed",
            "retcode": 1400,
            "message": "denied",
            "data": serde_json::Value::Null,
        })
        .to_string(),
        HttpReply::InvalidJson => "not-json".to_string(),
    };
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(resp.as_bytes()).await?;
    Ok(())
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_content_length(header: &str) -> usize {
    header
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once(':')?;
            if k.eq_ignore_ascii_case("content-length") {
                v.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

async fn run_event_ws(mut ws: WebSocketStream<TcpStream>, mut cmd_rx: mpsc::Receiver<ConnCmd>) {
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(ConnCmd::Close) => {
                        let _ = ws.close(None).await;
                        return;
                    }
                    Some(ConnCmd::Send(msg)) => {
                        if ws.send(msg).await.is_err() {
                            return;
                        }
                    }
                    Some(ConnCmd::Drop) | None => return,
                }
            }
            msg = ws.next() => {
                match msg {
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

pub(crate) fn conf() -> KoviConf {
    KoviConf::new(ID::new(1i64), None, false)
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

pub(crate) async fn bot_after_event_disconnect(how: Disconnect) -> RunOutcome {
    let _guard = status_file_guard();
    let server = MockMilky::start().await;
    let driver = server.driver();
    let handle = tokio::spawn(Bot::build(conf(), driver).run());
    server.wait_event().await;
    server.disconnect_event(how).await;
    observe_run(handle).await
}

pub(crate) async fn event_stream_after_disconnect(how: Disconnect) -> StreamOutcome {
    let server = MockMilky::start().await;
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

pub(crate) fn driver_on_port(port: u16) -> MilkyDriver {
    MilkyDriver::new(MilkyDriverConfig {
        server: Server::new(
            Host::IpAddr("127.0.0.1".parse().expect("ip")),
            port,
            String::new(),
            false,
            "/".into(),
        ),
    })
}
