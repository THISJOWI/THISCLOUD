use super::EventBus;
use async_trait::async_trait;

#[async_trait]
pub trait Module: Send {
    fn name(&self) -> &str;
    async fn start(&mut self, event_bus: &EventBus) -> anyhow::Result<()>;
    async fn stop(&mut self) -> anyhow::Result<()>;
    fn is_running(&self) -> bool;
}

pub struct ModuleManager {
    modules: Vec<Box<dyn Module>>,
}

impl ModuleManager {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn register(&mut self, module: Box<dyn Module>) {
        self.modules.push(module);
    }

    pub fn module_names(&self) -> Vec<&str> {
        self.modules.iter().map(|m| m.name()).collect()
    }

    pub async fn start_all(&mut self, event_bus: &EventBus) -> anyhow::Result<()> {
        for module in &mut self.modules {
            tracing::info!("Starting module: {}", module.name());
            module.start(event_bus).await?;
        }
        Ok(())
    }

    pub async fn stop_all(&mut self) -> anyhow::Result<()> {
        for module in self.modules.iter_mut().rev() {
            tracing::info!("Stopping module: {}", module.name());
            module.stop().await?;
        }
        Ok(())
    }
}

impl Default for ModuleManager {
    fn default() -> Self {
        Self::new()
    }
}
