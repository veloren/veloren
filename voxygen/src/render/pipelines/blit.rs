pub struct BindGroup {
    pub(in super::super) bind_group: wgpu::BindGroup,
}

pub struct BlitLayout {
    pub layout: wgpu::BindGroupLayout,
}

impl BlitLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            layout: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // Color source
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            filtering: true,
                            comparison: false,
                        },
                        count: None,
                    },
                ],
            }),
        }
    }

    pub fn bind(
        &self,
        device: &wgpu::Device,
        src_color: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> BindGroup {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        BindGroup { bind_group }
    }
}

pub struct BlitPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl BlitPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        sc_desc: &wgpu::SwapChainDescriptor,
        layout: &BlitLayout,
    ) -> Self {
        common_base::span!(_guard, "BlitPipeline::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Blit pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&layout.layout],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: "main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    blend: None,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        Self {
            pipeline: render_pipeline,
        }
    }
}
