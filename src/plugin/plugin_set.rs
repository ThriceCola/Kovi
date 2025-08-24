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
}
