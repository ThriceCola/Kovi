use crate::plugin::Plugin;

#[derive(Clone, Default)]
pub struct PluginSet {
    pub set: Vec<Plugin>,
}

impl PluginSet {
    pub fn new() -> Self {
        Self { set: Vec::new() }
    }

    pub fn with(mut self, plugin: Plugin) -> Self {
        self.set.push(plugin);
        self
    }

    pub fn push(&mut self, plugin: Plugin) {
        self.set.push(plugin);
    }
}

#[macro_export]
macro_rules! plugins {
    ($( $plugin:ident ),* $(,)* ) => {
        {
            let mut set = kovi::plugin::plugin_set::PluginSet::new();
            $(
                let plugin = $plugin::__kovi_build_plugin();
                kovi::log::info!("Mounting plugin: {}", &plugin.name);
                set.push(plugin);
            )*
            set
        }
    };
}
