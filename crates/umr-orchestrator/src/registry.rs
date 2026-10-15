use umr_core::{
    BackendCapabilities, CapabilityNegotiator, IAndroidBackend, IIosBackend, NegotiationResult,
    PackageMetadata,
};

/// In-memory registry of available execution backends.
pub struct BackendRegistry {
    android_backends: Vec<Box<dyn IAndroidBackend>>,
    ios_backends: Vec<Box<dyn IIosBackend>>,
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self {
            android_backends: Vec::new(),
            ios_backends: Vec::new(),
        }
    }

    pub fn register_android(&mut self, backend: Box<dyn IAndroidBackend>) {
        self.android_backends.push(backend);
    }

    pub fn register_ios(&mut self, backend: Box<dyn IIosBackend>) {
        self.ios_backends.push(backend);
    }

    pub fn get_android_mut(&mut self, id: &str) -> Option<&mut Box<dyn IAndroidBackend>> {
        self.android_backends
            .iter_mut()
            .find(|b| b.id() == id)
    }

    pub fn get_android(&self, id: &str) -> Option<&Box<dyn IAndroidBackend>> {
        self.android_backends
            .iter()
            .find(|b| b.id() == id)
    }

    pub fn get_ios_mut(&mut self, id: &str) -> Option<&mut Box<dyn IIosBackend>> {
        self.ios_backends
            .iter_mut()
            .find(|b| b.id() == id)
    }

    pub fn get_ios(&self, id: &str) -> Option<&Box<dyn IIosBackend>> {
        self.ios_backends
            .iter()
            .find(|b| b.id() == id)
    }

    pub fn negotiate(&self, package: &PackageMetadata, is_macos: bool) -> NegotiationResult {
        let android_refs: Vec<&dyn IAndroidBackend> =
            self.android_backends.iter().map(|b| b.as_ref()).collect();
        let ios_refs: Vec<&dyn IIosBackend> =
            self.ios_backends.iter().map(|b| b.as_ref()).collect();

        CapabilityNegotiator::negotiate(package, &android_refs, &ios_refs, is_macos)
    }

    pub fn list_capabilities(&self) -> Vec<BackendCapabilities> {
        let mut list = Vec::new();
        for b in &self.android_backends {
            list.push(b.capabilities().clone());
        }
        for b in &self.ios_backends {
            list.push(b.capabilities().clone());
        }
        list
    }
}

