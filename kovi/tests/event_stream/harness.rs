use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use kovi::ExitEvent;
use kovi::bot::{ApiReturn, Bot, SendApi};
use kovi::config::kovi_conf::KoviConf;
use kovi::driver::{AnyError, ApiHandlerResult, Driver, DriverEvent, MessageEventRegister};
use kovi::event::id::ID;
use kovi::event::{Event, MessageEventTrait};
use kovi::futures_util::stream;
use tokio::sync::{Mutex, Notify, mpsc};

pub(crate) const HANG_TIMEOUT: Duration = Duration::from_millis(800);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunOutcome {
    ExitedFromDrive,
    ExitedFromSignal,
    Hung,
    Panicked,
}

pub(crate) struct DummyMsg;

impl Event for DummyMsg {
    fn de(
        _: &kovi::event::InternalEvent,
        _: &kovi::bot::BotInformation,
        _: &mpsc::Sender<kovi::types::ApiAndOptOneshot>,
    ) -> Option<Self> {
        None
    }
}

impl MessageEventTrait for DummyMsg {
    fn get_sender_name(&self) -> Option<&str> {
        unreachable!()
    }
    fn get_sender_id(&self) -> kovi::event::id::ref_id::RefID<'_> {
        unreachable!()
    }
    fn get_message(&self) -> &kovi::message::Message {
        unreachable!()
    }
    fn get_message_type_str(&self) -> Option<&str> {
        None
    }
    fn get_group_id(&self) -> Option<kovi::event::id::ref_id::RefID<'_>> {
        None
    }
}

pub(crate) struct MockDriver {
    event_rx: Mutex<Option<mpsc::Receiver<Result<DriverEvent, AnyError>>>>,
    ready: Arc<Notify>,
}

impl MockDriver {
    pub(crate) fn new() -> (
        Self,
        mpsc::Sender<Result<DriverEvent, AnyError>>,
        Arc<Notify>,
    ) {
        let (tx, rx) = mpsc::channel(16);
        let ready = Arc::new(Notify::new());
        (
            Self {
                event_rx: Mutex::new(Some(rx)),
                ready: ready.clone(),
            },
            tx,
            ready,
        )
    }
}

#[async_trait]
impl Driver for MockDriver {
    async fn event_channel(
        &self,
    ) -> Result<
        std::pin::Pin<
            Box<dyn kovi::futures_util::Stream<Item = Result<DriverEvent, AnyError>> + Send>,
        >,
        AnyError,
    > {
        let rx = self
            .event_rx
            .lock()
            .await
            .take()
            .ok_or("event_channel called twice")?;
        self.ready.notify_one();
        let stream = stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        });
        Ok(Box::pin(stream))
    }

    fn api_handler(&self, _value: SendApi) -> ApiHandlerResult {
        Box::pin(async {
            Ok(Ok(ApiReturn {
                status: "ok".into(),
                retcode: 0,
                message: None,
                data: kovi::serde_json::json!({}),
            }))
        })
    }

    fn message_event_register(&self) -> MessageEventRegister {
        MessageEventRegister::register::<DummyMsg>()
    }
}

pub(crate) struct FailingChannelDriver;

#[async_trait]
impl Driver for FailingChannelDriver {
    async fn event_channel(
        &self,
    ) -> Result<
        std::pin::Pin<
            Box<dyn kovi::futures_util::Stream<Item = Result<DriverEvent, AnyError>> + Send>,
        >,
        AnyError,
    > {
        Err("connection refused".into())
    }

    fn api_handler(&self, _value: SendApi) -> ApiHandlerResult {
        Box::pin(async {
            Ok(Ok(ApiReturn {
                status: "ok".into(),
                retcode: 0,
                message: None,
                data: kovi::serde_json::json!({}),
            }))
        })
    }

    fn message_event_register(&self) -> MessageEventRegister {
        MessageEventRegister::register::<DummyMsg>()
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

pub(crate) fn spawn_bot(driver: MockDriver) -> tokio::task::JoinHandle<ExitEvent> {
    tokio::spawn(Bot::build(conf(), driver).run())
}

pub(crate) async fn wait_ready(notified: impl std::future::Future<Output = ()>) {
    tokio::time::timeout(Duration::from_secs(3), notified)
        .await
        .expect("event_channel was not opened");
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
