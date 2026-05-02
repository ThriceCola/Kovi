use crate::event::{
    AdminMsgEvent, GroupMsgEvent, MsgEvent, MsgSendFromServerEvent, NoticeEvent, PrivateMsgEvent,
    RequestEvent,
};
use kovi::PluginBuilder;
use std::sync::Arc;

pub trait EventRegistrar {
    /// 注册事件处理函数。
    fn on_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<MsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<MsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_admin_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<AdminMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<AdminMsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_private_msg<F, Fut>(handler: F)
    where
        F: Fn(Arc<PrivateMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<PrivateMsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_group_msg<F, Fut>(handler: F)
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
    fn on_msg_send<F, Fut>(handler: F)
    where
        F: Fn(Arc<MsgSendFromServerEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<MsgSendFromServerEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_notice<F, Fut>(handler: F)
    where
        F: Fn(Arc<NoticeEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<NoticeEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_request<F, Fut>(handler: F)
    where
        F: Fn(Arc<RequestEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<RequestEvent, _>(handler)
    }
}

impl EventRegistrar for PluginBuilder {
}
