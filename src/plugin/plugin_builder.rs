use crate::RT;
use crate::bot::BotInformation;
use crate::bot::Host;
use crate::bot::plugin_builder::event::Event;
use crate::bot::{Bot, runtimebot::RuntimeBot};
use crate::event::InternalEvent;
use crate::event::MsgSendFromServerEvent;
use crate::event::{AdminMsgEvent, GroupMsgEvent, PrivateMsgEvent};
use crate::plugin::{PLUGIN_BUILDER, PLUGIN_NAME};
use crate::types::{ApiAndOneshot, NoArgsFn, PinFut};
use croner::Cron;
use croner::errors::CronError;
use event::{MsgEvent, NoticeEvent, RequestEvent};
use log::error;
use parking_lot::RwLock;
use std::any::Any;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::mpsc;

// 兼容旧版本
pub use crate::bot::event;

macro_rules! assert_right_place {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(_) => panic!("Using PluginBuilder in wrong place"),
        }
    };
}

pub(crate) trait DowncastArc: Any {
    fn downcast_arc<T: Any>(self: Arc<Self>) -> Result<Arc<T>, Arc<Self>>;
}
impl<T: ?Sized + Any> DowncastArc for T {
    fn downcast_arc<U: Any>(self: Arc<Self>) -> Result<Arc<U>, Arc<Self>> {
        if (*self).type_id() == std::any::TypeId::of::<U>() {
            let raw: *const Self = Arc::into_raw(self);
            Ok(unsafe { Arc::from_raw(raw as *const U) })
        } else {
            Err(self)
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct Listen {
    pub(crate) list: Vec<Arc<ListenInner>>,
    pub(crate) drop: Vec<NoArgsFn>,
}
impl Listen {
    pub(crate) fn clear(&mut self) {
        self.list.clear();
        self.drop.clear();
        self.list.shrink_to_fit();
        self.drop.shrink_to_fit();
    }
}
type ArcTypeDeFn = Arc<
    dyn Fn(&InternalEvent, &BotInformation, &mpsc::Sender<ApiAndOneshot>) -> Option<Arc<dyn Event>>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub(crate) struct ListenInner {
    pub(crate) type_id: std::any::TypeId,
    pub(crate) type_de: ArcTypeDeFn,
    pub(crate) handler: Arc<dyn Fn(Arc<dyn Event>) -> PinFut + Send + Sync>,
}

impl Listen {
    pub(crate) fn on<T, F, Fut>(&mut self, handler: F)
    where
        T: Event,
        F: Fn(Arc<T>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        let handler = Arc::new(handler);

        self.list.push(Arc::new(ListenInner {
            type_id: std::any::TypeId::of::<T>(),
            type_de: Arc::new(|value, bot_info, sender| {
                Some(Arc::new(T::de(value, bot_info, sender)?))
            }),
            handler: Arc::new(move |evt: Arc<dyn Event>| {
                let downcasted = evt.downcast_arc::<T>();

                match downcasted {
                    Ok(downcasted) => Box::pin({
                        let handler = handler.clone();
                        async move {
                            handler(downcasted).await;
                        }
                    }),
                    Err(_) => Box::pin(async {}),
                }
            }),
        }));
    }
}

#[derive(Clone)]
pub struct PluginBuilder {
    pub(crate) bot: Arc<RwLock<Bot>>,
    pub(crate) runtime_bot: Arc<RuntimeBot>,
}

impl PluginBuilder {
    pub(crate) fn new(
        name: String,
        bot: Arc<RwLock<Bot>>,
        host: Host,
        port: u16,
        api_tx: mpsc::Sender<ApiAndOneshot>,
    ) -> Self {
        let bot_weak = Arc::downgrade(&bot);

        let runtime_bot = Arc::new(RuntimeBot {
            host,
            port,

            bot: bot_weak,
            plugin_name: name,
            api_tx,
        });

        PluginBuilder { bot, runtime_bot }
    }

    pub fn get_runtime_bot() -> Arc<RuntimeBot> {
        assert_right_place!(PLUGIN_BUILDER.try_with(|p| p.runtime_bot.clone()))
    }

    pub fn get_plugin_name() -> String {
        assert_right_place!(PLUGIN_BUILDER.try_with(|p| p.runtime_bot.plugin_name.to_string()))
    }

    pub fn get_plugin_host() -> (Host, u16) {
        assert_right_place!(
            PLUGIN_BUILDER.try_with(|p| (p.runtime_bot.host.clone(), p.runtime_bot.port))
        )
    }
}

impl PluginBuilder {
    pub fn on<T: Event, Fut>(handler: impl Fn(Arc<T>) -> Fut + Send + Sync + 'static)
    where
        Fut: Future + Send,
        Fut::Output: Send,
    {
        assert_right_place!(PLUGIN_BUILDER.try_with(|p| {
            let mut bot = p.bot.write();
            let bot_plugin = bot.plugins.get_mut(&p.runtime_bot.plugin_name).expect("");

            bot_plugin.listen.on(handler);
        }));
    }

    /// 注册事件处理函数。
    pub fn on_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<MsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<MsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    pub fn on_admin_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AdminMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<AdminMsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    pub fn on_private_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<PrivateMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<PrivateMsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    pub fn on_group_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<GroupMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<GroupMsgEvent, _>(handler)
    }

    #[deprecated(
        note = "请使用 `PluginBuilder::on::(|event: Arc<MsgSendFromServerEvent>| fn())` 代替"
    )]
    /// 注册事件处理函数。
    pub fn on_msg_send<F, Fut>(handler: F)
    where
        F: Fn(Arc<MsgSendFromServerEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<MsgSendFromServerEvent, _>(handler)
    }

    /// 注册事件处理函数。
    pub fn on_notice<F, Fut>(handler: F)
    where
        F: Fn(Arc<NoticeEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<NoticeEvent, _>(handler)
    }

    /// 注册事件处理函数。
    pub fn on_request<F, Fut>(handler: F)
    where
        F: Fn(Arc<RequestEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<RequestEvent, _>(handler)
    }

    #[deprecated(note = "请使用 `on_notice` 代替")]
    /// 注册事件处理函数。
    pub fn on_all_notice<F, Fut>(handler: F)
    where
        F: Fn(Arc<NoticeEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        Self::on_notice(handler)
    }

    #[deprecated(note = "请使用 `on_request` 代替")]
    /// 注册事件处理函数。
    pub fn on_all_request<F, Fut>(handler: F)
    where
        F: Fn(Arc<RequestEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        Self::on_request(handler)
    }

    /// 注册程序结束事件处理函数。
    ///
    /// 注册处理程序，用于处理接收到的程序结束事件。
    pub fn drop<F, Fut>(handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        assert_right_place!(PLUGIN_BUILDER.try_with(|p| {
            let mut bot = p.bot.write();
            let bot_plugin = bot
                .plugins
                .get_mut(&p.runtime_bot.plugin_name)
                .expect("unreachable");

            bot_plugin.listen.drop.push(Arc::new({
                let handler = Arc::new(handler);
                move || {
                    Box::pin({
                        let handler = handler.clone();
                        async move {
                            handler().await;
                        }
                    })
                }
            }));
        }));
    }

    /// 注册定时任务。
    ///
    /// 传入 Cron 。
    pub fn cron<F, Fut>(cron: &str, handler: F) -> Result<(), CronError>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        assert_right_place!(PLUGIN_BUILDER.try_with(|p| {
            let cron = match Cron::new(cron).with_seconds_optional().parse() {
                Ok(v) => v,
                Err(e) => return Err(e),
            };
            Self::run_cron_task(p, cron, handler);
            Ok(())
        }))
    }

    /// 注册定时任务。
    ///
    /// 传入 Cron 。
    pub fn cron_use_croner<F, Fut>(cron: Cron, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        assert_right_place!(PLUGIN_BUILDER.try_with(|p| {
            Self::run_cron_task(p, cron, handler);
        }));
    }

    fn run_cron_task<F, Fut>(p: &PluginBuilder, cron: Cron, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        let name = Arc::new(p.runtime_bot.plugin_name.clone());
        let mut enabled = {
            let bot = p.bot.read();
            let plugin = bot.plugins.get(&*name).expect("unreachable");
            plugin.enabled.subscribe()
        };
        RT.spawn(PLUGIN_NAME.scope(name.clone(), async move {

            tokio::select! {
                _ = async {
                        loop {
                            let now = chrono::Local::now();
                            let next = match cron.find_next_occurrence(&now, false) {
                                Ok(v) => v,
                                Err(e) => {
                                    error!("{name} cron task error: {e}");
                                    break;
                                }
                            };
                            let time = next - now;
                            let duration = std::time::Duration::from_millis(time.num_milliseconds() as u64);
                            tokio::time::sleep(duration).await;
                            handler().await;
                        }
                } => {}
                _ = async {
                        loop {
                            enabled.changed().await.expect("The enabled channel closed");
                            if !*enabled.borrow_and_update() {
                                break;
                            }
                        }
                } => {}
            }
        }));
    }
}

#[macro_export]
macro_rules! async_move {
    // 匹配没有事件参数的情况
    (;$($var:ident),*; $($body:tt)*) => {
        {
            $(let $var = $var.clone();)*
            move || {
                $(let $var = $var.clone();)*
                async move
                    $($body)*
            }
        }
    };

    // 匹配有事件参数的情况
    ($event:ident; $($var:ident),*; $($body:tt)*) => {
        {
            $(let $var = $var.clone();)*
            move |$event| {
                $(let $var = $var.clone();)*
                async move
                    $($body)*
            }
        }
    };

    // 匹配只要一次clone的情况（自己tokio::spawn一个新线程）
    ($($var:ident),*;$($body:tt)*) => {
        {
            $(let $var = $var.clone();)*
            async move
                $($body)*
        }
    };
}
