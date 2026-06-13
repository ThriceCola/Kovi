use crate::event::{
    AdminMsgEvent, GroupMsgEvent, MsgEvent, MsgSendFromServerEvent, NoticeEvent, PrivateMsgEvent,
    RequestEvent,
};
use kovi::PluginBuilder;
use std::sync::Arc;

pub trait EventRegistrar {
    /// 注册事件处理函数。
    fn on_msg<F>(handler: F)
    where
        F: AsyncFn(Arc<MsgEvent>) + Send + Sync + 'static,
        for<'a> <F as AsyncFnMut<(Arc<MsgEvent>,)>>::CallRefFuture<'a>: std::marker::Send,
    {
        PluginBuilder::on::<MsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_admin_msg<F>(handler: F)
    where
        F: AsyncFn(Arc<AdminMsgEvent>) + Send + Sync + 'static,
        for<'a> <F as AsyncFnMut<(Arc<AdminMsgEvent>,)>>::CallRefFuture<'a>: std::marker::Send,
    {
        PluginBuilder::on::<AdminMsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_private_msg<F>(handler: F)
    where
        F: AsyncFn(Arc<PrivateMsgEvent>) + Send + Sync + 'static,
        for<'a> <F as AsyncFnMut<(Arc<PrivateMsgEvent>,)>>::CallRefFuture<'a>: std::marker::Send,
    {
        PluginBuilder::on::<PrivateMsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_group_msg<F>(handler: F)
    where
        F: AsyncFn(Arc<GroupMsgEvent>) + Send + Sync + 'static,
        for<'a> <F as AsyncFnMut<(Arc<GroupMsgEvent>,)>>::CallRefFuture<'a>: std::marker::Send,
    {
        PluginBuilder::on::<GroupMsgEvent, _>(handler)
    }

    #[deprecated(
        note = "请使用 `PluginBuilder::on::(|event: Arc<MsgSendFromServerEvent>| fn())` 代替"
    )]
    /// 注册事件处理函数。
    fn on_msg_send<F>(handler: F)
    where
        F: AsyncFn(Arc<MsgSendFromServerEvent>) + Send + Sync + 'static,
        for<'a> <F as AsyncFnMut<(Arc<MsgSendFromServerEvent>,)>>::CallRefFuture<'a>:
            std::marker::Send,
    {
        PluginBuilder::on::<MsgSendFromServerEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_notice<F>(handler: F)
    where
        F: AsyncFn(Arc<NoticeEvent>) + Send + Sync + 'static,
        for<'a> <F as AsyncFnMut<(Arc<NoticeEvent>,)>>::CallRefFuture<'a>: std::marker::Send,
    {
        PluginBuilder::on::<NoticeEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_request<F>(handler: F)
    where
        F: AsyncFn(Arc<RequestEvent>) + Send + Sync + 'static,
        for<'a> <F as AsyncFnMut<(Arc<RequestEvent>,)>>::CallRefFuture<'a>: std::marker::Send,
    {
        PluginBuilder::on::<RequestEvent, _>(handler)
    }
}

impl EventRegistrar for PluginBuilder {
}
