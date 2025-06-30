#[allow(dead_code)]
use crate::core::provider::DNSProvider;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn DNSProvider>>,
}

#[allow(dead_code)]
impl ProviderRegistry {
    pub fn new() -> Self {
        ProviderRegistry {
            providers: HashMap::new(),
        }
    }

    pub fn register(&mut self, provider: Arc<dyn DNSProvider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn DNSProvider>> {
        self.providers.get(name).cloned()
    }

    pub fn list(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}
