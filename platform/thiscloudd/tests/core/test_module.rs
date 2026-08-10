use std::sync::atomic::{AtomicBool, Ordering};

use thiscloudd::core::Module;

struct TestModule {
    name: String,
    started: AtomicBool,
}

#[async_trait::async_trait]
impl thiscloudd::core::Module for TestModule {
    fn name(&self) -> &str {
        &self.name
    }

    async fn start(&mut self, _event_bus: &thiscloudd::core::EventBus) -> anyhow::Result<()> {
        self.started.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        self.started.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.started.load(Ordering::SeqCst)
    }
}

#[tokio::test]
async fn test_module_lifecycle() {
    let event_bus = thiscloudd::core::EventBus::new();
    let mut module = TestModule {
        name: "test-module".to_string(),
        started: AtomicBool::new(false),
    };

    assert!(!module.is_running());

    module.start(&event_bus).await.unwrap();
    assert!(module.is_running());

    module.stop().await.unwrap();
    assert!(!module.is_running());
}

#[tokio::test]
async fn test_module_manager() {
    let event_bus = thiscloudd::core::EventBus::new();
    let mut manager = thiscloudd::core::ModuleManager::new();

    let module = TestModule {
        name: "test-module".to_string(),
        started: AtomicBool::new(false),
    };

    manager.register(Box::new(module));
    assert_eq!(manager.module_names().len(), 1);

    manager.start_all(&event_bus).await.unwrap();
    manager.stop_all().await.unwrap();
}

#[tokio::test]
async fn test_module_manager_multiple() {
    let event_bus = thiscloudd::core::EventBus::new();
    let mut manager = thiscloudd::core::ModuleManager::new();

    for i in 0..3 {
        manager.register(Box::new(TestModule {
            name: format!("module-{}", i),
            started: AtomicBool::new(false),
        }));
    }

    assert_eq!(manager.module_names().len(), 3);
    assert_eq!(manager.module_names()[0], "module-0");
    assert_eq!(manager.module_names()[2], "module-2");

    manager.start_all(&event_bus).await.unwrap();
    manager.stop_all().await.unwrap();
}
