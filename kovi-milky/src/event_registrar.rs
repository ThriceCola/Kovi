use crate::event::{AdminMsgEvent, FriendMsgEvent, GroupMsgEvent, MsgEvent};
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
        F: Fn(Arc<FriendMsgEvent>) -> Fut + Send + Sync + 'static,
        Fut: Future + Send,
        Fut::Output: Send,
    {
        PluginBuilder::on::<FriendMsgEvent, _>(handler)
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
}

impl EventRegistrar for PluginBuilder {
}
