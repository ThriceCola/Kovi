use crate::bot::Bot;
use crate::bot::runtimebot::RuntimeBot;
use crate::event::Event;
use crate::plugin::{PLUGIN_BUILDER, PLUGIN_NAME};
use crate::types::{ApiAndOptOneshot, ArcTypeDeFn, NoArgsFn, PinFut};
use croner::Cron;
use croner::errors::CronError;
use log::error;
use parking_lot::RwLock;
use std::any::Any;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::mpsc;

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
                    Err(_) => panic!("Type downcasted error!"),
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
        api_tx: mpsc::Sender<ApiAndOptOneshot>,
    ) -> Self {
        let bot_weak = Arc::downgrade(&bot);

        let runtime_bot = Arc::new(RuntimeBot {
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

    // pub fn get_plugin_host() -> (Host, u16) {
    //     assert_right_place!(
    //         PLUGIN_BUILDER.try_with(|p| (p.runtime_bot.host.clone(), p.runtime_bot.port))
    //     )
    // }
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
        tokio::spawn(PLUGIN_NAME.scope(name.clone(), async move {

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
