pub struct Context {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Context {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .unwrap();

        #[cfg(feature = "profiling")]
        let descriptor = wgpu::DeviceDescriptor {
            required_features: adapter.features()
                & wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES,
            ..Default::default()
        };

        #[cfg(not(feature = "profiling"))]
        let descriptor = wgpu::DeviceDescriptor::default();

        let (device, queue) = adapter.request_device(&descriptor).await.unwrap();

        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }
}
