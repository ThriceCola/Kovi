use crate::event::{AdminMsgEvent, FriendMsgEvent, GroupMsgEvent, MsgEvent};
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
        F: AsyncFn(Arc<FriendMsgEvent>) + Send + Sync + 'static,
        for<'a> <F as AsyncFnMut<(Arc<FriendMsgEvent>,)>>::CallRefFuture<'a>: std::marker::Send,
    {
        PluginBuilder::on::<FriendMsgEvent, _>(handler)
    }

    /// 注册事件处理函数。
    fn on_group_msg<F>(handler: F)
    where
        F: AsyncFn(Arc<GroupMsgEvent>) + Send + Sync + 'static,
        for<'a> <F as AsyncFnMut<(Arc<GroupMsgEvent>,)>>::CallRefFuture<'a>: std::marker::Send,
    {
        PluginBuilder::on::<GroupMsgEvent, _>(handler)
    }
}

impl EventRegistrar for PluginBuilder {
}
