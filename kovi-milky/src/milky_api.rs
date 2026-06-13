pub mod file;
pub mod friend;
pub mod group;
pub mod message;
pub mod system;

pub use file::MilkyFileApi;
pub use friend::MilkyFriendApi;
pub use group::MilkyGroupApi;
pub use message::MilkyMessageApi;
pub use system::MilkySystemApi;

use kovi::RuntimeBot;
use kovi::bot::runtimebot::CanSendApi;

/// MilkyTrait 整合了所有 Milky API 分类 trait。
///
/// 实现对分类trait的统一继承,方便用户使用。
pub trait MilkyTrait:
    CanSendApi + MilkySystemApi + MilkyMessageApi + MilkyFriendApi + MilkyGroupApi + MilkyFileApi
{
}

impl<T> MilkyTrait for T where
    T: CanSendApi
        + MilkySystemApi
        + MilkyMessageApi
        + MilkyFriendApi
        + MilkyGroupApi
        + MilkyFileApi
{
}

impl MilkySystemApi for RuntimeBot {
}
impl MilkyMessageApi for RuntimeBot {
}
impl MilkyFriendApi for RuntimeBot {
}
impl MilkyGroupApi for RuntimeBot {
}
impl MilkyFileApi for RuntimeBot {
}
