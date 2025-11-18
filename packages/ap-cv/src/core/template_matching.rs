//! Template matching implementation based on compute shader through wgpu.
//!
//! Currently only supports grayscale image.
use std::{
    fmt::Display,
    sync::{Arc, Mutex, OnceLock},
};

#[cfg(feature = "profiling")]
use wgpu_profiler::{GpuProfiler, GpuProfilerSettings};

#[cfg(feature = "profiling")]
// Since the timing information we get from WGPU may be several frames behind the CPU, we can't report these frames to
// the singleton returned by `puffin::GlobalProfiler::lock`. Instead, we need our own `puffin::GlobalProfiler` that we
// can be several frames behind puffin's main global profiler singleton.
static PUFFIN_GPU_PROFILER: std::sync::LazyLock<Mutex<puffin::GlobalProfiler>> =
    std::sync::LazyLock::new(|| Mutex::new(puffin::GlobalProfiler::default()));

use bytemuck::{Pod, Zeroable};
use image::{ImageBuffer, Luma};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupLayoutDescriptor, BufferDescriptor, BufferUsages,
    CommandEncoderDescriptor, PipelineLayoutDescriptor, include_wgsl, util::DeviceExt,
};

use crate::gpu::Context;

#[derive(Clone, Debug)]
pub struct Match {
    pub location: (u32, u32),
    pub value: f32,
}

pub use imageproc::template_matching::find_extremes;

pub fn find_matches(
    input: &ImageBuffer<Luma<f32>, Vec<f32>>,
    template_width: u32,
    template_height: u32,
    method: MatchTemplateMethod,
    threshold: f32,
) -> Vec<Match> {
    let mut matches: Vec<Match> = Vec::new();

    for (x, y, p) in input.enumerate_pixels() {
        let value = p.0[0];
        if is_a_more_match_than_b(value, threshold, method) {
            if let Some(m) = matches.iter_mut().rev().find(|m| {
                ((m.location.0 as i32 - x as i32).abs() as u32) < template_width
                    && ((m.location.1 as i32 - y as i32).abs() as u32) < template_height
            }) {
                if is_a_more_match_than_b(value, m.value, method) {
                    m.location = (x, y);
                    m.value = value;
                }
                continue;
            } else {
                matches.push(Match {
                    location: (x, y),
                    value,
                });
            }
        }
    }

    // sort matches by value (is_x_more_match_than_y)
    matches.sort_by(|a, b| {
        if is_a_more_match_than_b(a.value, b.value, method) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    matches
}

pub fn is_a_more_match_than_b(a: f32, b: f32, method: MatchTemplateMethod) -> bool {
    if matches!(
        method,
        MatchTemplateMethod::SumOfSquaredDifference
            | MatchTemplateMethod::SumOfSquaredDifferenceNormed
    ) {
        return a < b;
    } else {
        return a > b;
    };
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MatchTemplateMethod {
    SumOfSquaredDifference,
    SumOfSquaredDifferenceNormed,
    CrossCorrelation,
    CrossCorrelationNormed,
    CorrelationCoefficient,
    CorrelationCoefficientNormed,
}

impl MatchTemplateMethod {
    pub const ALL: [MatchTemplateMethod; 6] = [
        MatchTemplateMethod::SumOfSquaredDifference,
        MatchTemplateMethod::SumOfSquaredDifferenceNormed,
        MatchTemplateMethod::CrossCorrelation,
        MatchTemplateMethod::CrossCorrelationNormed,
        MatchTemplateMethod::CorrelationCoefficient,
        MatchTemplateMethod::CorrelationCoefficientNormed,
    ];
}

impl Display for MatchTemplateMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MatchTemplateMethod::SumOfSquaredDifference => "sqdiff",
            MatchTemplateMethod::SumOfSquaredDifferenceNormed => "sqdiff_normed",
            MatchTemplateMethod::CrossCorrelation => "ccorr",
            MatchTemplateMethod::CrossCorrelationNormed => "ccorr_normed",
            MatchTemplateMethod::CorrelationCoefficient => "ccoeff",
            MatchTemplateMethod::CorrelationCoefficientNormed => "ccoeff_normed",
        };
        f.write_str(s)
    }
}

pub fn match_template(
    image: &ImageBuffer<Luma<f32>, Vec<f32>>,
    template: &ImageBuffer<Luma<f32>, Vec<f32>>,
    method: MatchTemplateMethod,
    padding: bool,
) -> ImageBuffer<Luma<f32>, Vec<f32>> {
    let mut matcher = matcher().lock().unwrap();
    matcher.match_template(image, template, method, padding)
}

/// internal
fn matcher() -> &'static Arc<Mutex<Matcher>> {
    static MATCHER: OnceLock<Arc<Mutex<Matcher>>> = OnceLock::new();
    MATCHER.get_or_init(|| Arc::new(Mutex::new(Matcher::new())))
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Uniforms {
    image_width: u32,
    image_height: u32,
    template_width: u32,
    template_height: u32,
}

struct Matcher {
    ctx: Context,

    input_buffer: Option<wgpu::Buffer>,
    template_buffer: Option<wgpu::Buffer>,
    result_buffer: Option<wgpu::Buffer>,
    staging_buffer: Option<wgpu::Buffer>,
    uniform_buffer: wgpu::Buffer,

    bind_group_layout: wgpu::BindGroupLayout,
    // pipeline_layout: wgpu::PipelineLayout,
    bind_group: Option<wgpu::BindGroup>,
    pipeline_ccorr: wgpu::ComputePipeline,
    pipeline_ccorr_normed: wgpu::ComputePipeline,
    pipeline_sqdiff: wgpu::ComputePipeline,
    pipeline_sqdiff_normed: wgpu::ComputePipeline,
    pipeline_ccoeff: wgpu::ComputePipeline,
    pipeline_ccoeff_normed: wgpu::ComputePipeline,

    #[cfg(feature = "profiling")]
    profiler: GpuProfiler,
}

impl Matcher {
    fn new() -> Self {
        let ctx = pollster::block_on(Context::new());
        let Context { device, .. } = &ctx;

        let bind_group_layout = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Matcher BindGroupLayout"),
                entries: &[
                    // input
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // template
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // result
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Matcher PipelineLayout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("uniform"),
            size: size_of::<Uniforms>() as _,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader_module =
            device.create_shader_module(include_wgsl!("../../shaders/template_matching.wgsl"));
        let pipeline_ccorr = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Cross Correlation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main_ccorr"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let pipeline_ccorr_normed =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Cross Correlation Normed Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: Some("main_ccorr_normed"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let pipeline_sqdiff = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Sum of Squared Difference Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main_sqdiff"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let pipeline_sqdiff_normed =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Sum of Squared Difference Normed Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: Some("main_sqdiff_normed"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let pipeline_ccoeff = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Correlation Coefficient Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main_ccoeff"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let pipeline_ccoeff_normed =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Correlation Coefficient Normed Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: Some("main_ccoeff_normed"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        #[cfg(feature = "profiling")]
        let profiler = GpuProfiler::new(&ctx.device, GpuProfilerSettings::default())
            .expect("Failed to create profiler");

        Matcher {
            ctx,
            input_buffer: None,
            template_buffer: None,
            result_buffer: None,
            staging_buffer: None,
            uniform_buffer,
            bind_group_layout,
            bind_group: None,
            // pipeline_layout,
            pipeline_ccorr,
            pipeline_ccorr_normed,
            pipeline_sqdiff,
            pipeline_sqdiff_normed,
            pipeline_ccoeff,
            pipeline_ccoeff_normed,
            #[cfg(feature = "profiling")]
            profiler,
        }
    }

    fn create_new_bind_group(&self) -> BindGroup {
        // println!("input buffer size: {:?}", self.input_buffer.as_ref().unwrap().size());
        // println!("template buffer size: {:?}", self.template_buffer.as_ref().unwrap().size());
        // println!("result buffer size: {:?}", self.result_buffer.as_ref().unwrap().size());
        // println!("staging buffer size: {:?}", self.staging_buffer.as_ref().unwrap().size());
        self.ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Matcher BindGroup"),
            layout: &self.bind_group_layout,
            entries: &[
                // input
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input_buffer.as_ref().unwrap().as_entire_binding(),
                },
                // template
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.template_buffer.as_ref().unwrap().as_entire_binding(),
                },
                // result
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.result_buffer.as_ref().unwrap().as_entire_binding(),
                },
                // uniform
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn match_template(
        &mut self,
        image: &ImageBuffer<Luma<f32>, Vec<f32>>,
        template: &ImageBuffer<Luma<f32>, Vec<f32>>,
        match_method: MatchTemplateMethod,
        padding: bool,
    ) -> ImageBuffer<Luma<f32>, Vec<f32>> {
        profiling::scope!("match_template");

        let (image, template) = if matches!(
            match_method,
            MatchTemplateMethod::CorrelationCoefficient
                | MatchTemplateMethod::CorrelationCoefficientNormed
        ) {
            let avg_kernel = ImageBuffer::from_pixel(
                template.width(),
                template.height(),
                Luma([1.0 / (template.width() * template.height()) as f32]),
            );
            let avg_image = self.match_template(
                image,
                &avg_kernel,
                MatchTemplateMethod::CrossCorrelation,
                true,
            );
            let avg_template = self.match_template(
                template,
                &avg_kernel,
                MatchTemplateMethod::CrossCorrelation,
                true,
            );

            let image = ImageBuffer::from_vec(
                image.width(),
                image.height(),
                image
                    .as_raw()
                    .iter()
                    .zip(avg_image.as_raw().iter())
                    .map(|(v, avg)| v - avg)
                    .collect(),
            )
            .unwrap();
            let template = ImageBuffer::from_vec(
                template.width(),
                template.height(),
                template
                    .as_raw()
                    .iter()
                    .zip(avg_template.as_raw().iter())
                    .map(|(v, avg)| v - avg)
                    .collect(),
            )
            .unwrap();

            (image, template)
        } else {
            (image.clone(), template.clone())
        };
        let image = if padding {
            let padded_image = ImageBuffer::from_fn(
                image.width() + template.width() - 1,
                image.height() + template.height() - 1,
                |x, y| {
                    if x >= image.width() || y >= image.height() {
                        Luma([0.0])
                    } else {
                        *image.get_pixel(x, y)
                    }
                },
            );
            padded_image
        } else {
            image.clone()
        };
        let image = &image;
        let template = &template;

        let (result_w, result_h) = (
            image.width() - template.width() + 1,
            image.height() - template.height() + 1,
        );
        let result_buf_sz = (result_w * result_h * size_of::<f32>() as u32) as u64;

        // update buffers
        let update = {
            profiling::scope!("update buffers");

            [
                prepare_buffer_init_with_image(
                    &self.ctx,
                    &mut self.input_buffer,
                    image,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                ),
                prepare_buffer_init_with_image(
                    &self.ctx,
                    &mut self.template_buffer,
                    template,
                    BufferUsages::STORAGE | BufferUsages::COPY_DST,
                ),
                prepare_buffer_init_with_size(
                    &self.ctx,
                    &mut self.result_buffer,
                    result_buf_sz,
                    BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
                ),
                prepare_buffer_init_with_size(
                    &self.ctx,
                    &mut self.staging_buffer,
                    result_buf_sz,
                    BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                ),
            ]
            .iter()
            .any(|x| *x)
        };

        // update bind_group and uniforms
        if update {
            profiling::scope!("update bind_group and uniforms");
            self.bind_group = Some(self.create_new_bind_group());
            // let template_sq_sum = template.as_raw().iter().map(|x| x * x).sum::<f32>();
            let uniforms = Uniforms {
                image_height: image.height(),
                image_width: image.width(),
                template_height: template.height(),
                template_width: template.width(),
                // template_sq_sum,
            };
            self.ctx
                .queue
                .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        }

        // Helper function to execute compute pass logic
        let encode_compute_pass = |pass: &mut wgpu::ComputePass<'_>| {
            pass.set_pipeline(match match_method {
                MatchTemplateMethod::CrossCorrelation => &self.pipeline_ccorr,
                MatchTemplateMethod::CrossCorrelationNormed => &self.pipeline_ccorr_normed,
                MatchTemplateMethod::SumOfSquaredDifference => &self.pipeline_sqdiff,
                MatchTemplateMethod::SumOfSquaredDifferenceNormed => &self.pipeline_sqdiff_normed,
                MatchTemplateMethod::CorrelationCoefficient => &self.pipeline_ccoeff,
                MatchTemplateMethod::CorrelationCoefficientNormed => &self.pipeline_ccoeff_normed,
            });
            pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
            pass.dispatch_workgroups(
                (result_w as f32 / 8.0).ceil() as u32,
                (result_h as f32 / 8.0).ceil() as u32,
                1,
            );
        };

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        {
            #[cfg(feature = "profiling")]
            let scope_label = format!("match_template_{}", match_method);
            #[cfg(feature = "profiling")]
            let mut scope = self.profiler.scope(&scope_label, &mut encoder);

            {
                let mut pass = {
                    #[cfg(feature = "profiling")]
                    {
                        scope.scoped_compute_pass("compute pass")
                    }
                    #[cfg(not(feature = "profiling"))]
                    {
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("compute pass"),
                            timestamp_writes: None,
                        })
                    }
                };
                encode_compute_pass(&mut pass);
            }

            // Copy buffer
            #[cfg(feature = "profiling")]
            {
                scope.recorder.copy_buffer_to_buffer(
                    self.result_buffer.as_ref().unwrap(),
                    0,
                    self.staging_buffer.as_ref().unwrap(),
                    0,
                    result_buf_sz,
                );
            }

            #[cfg(not(feature = "profiling"))]
            {
                encoder.copy_buffer_to_buffer(
                    self.result_buffer.as_ref().unwrap(),
                    0,
                    self.staging_buffer.as_ref().unwrap(),
                    0,
                    result_buf_sz,
                );
            }
        }

        #[cfg(feature = "profiling")]
        self.profiler.resolve_queries(&mut encoder);

        {
            profiling::scope!("submit encoder");
            self.ctx.queue.submit(Some(encoder.finish()));
        }

        #[cfg(feature = "profiling")]
        {
            self.profiler.end_frame().unwrap();
            // Query for oldest finished frame and report to puffin
            if let Some(results) = self
                .profiler
                .process_finished_frame(self.ctx.queue.get_timestamp_period())
            {
                let mut gpu_profiler = PUFFIN_GPU_PROFILER.lock().unwrap();
                wgpu_profiler::puffin::output_frame_to_puffin(&mut gpu_profiler, &results);
                gpu_profiler.new_frame();
            }
        }

        let res = {
            profiling::scope!("get output");
            // get output
            let buffer_slice = self.staging_buffer.as_ref().unwrap().slice(..);
            let (sender, receiver) = async_channel::bounded(1);
            buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.try_send(v).unwrap());

            self.ctx
                .device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();

            pollster::block_on(async {
                let result;

                if let Ok(()) = receiver.try_recv().unwrap() {
                    let data = buffer_slice.get_mapped_range();
                    result = bytemuck::cast_slice(&data).to_vec();
                    drop(data);
                    self.staging_buffer.as_ref().unwrap().unmap();
                } else {
                    result = vec![0.0; (result_w * result_h) as usize]
                };

                let res = ImageBuffer::from_vec(result_w, result_h, result).unwrap();

                res
            })
        };
        profiling::finish_frame!();
        res
    }
}

/// returns true if buffer is updated
fn prepare_buffer_init_with_size(
    ctx: &Context,
    buffer: &mut Option<wgpu::Buffer>,
    size: u64,
    usage: wgpu::BufferUsages,
) -> bool {
    let update = buffer.is_none() || buffer.as_ref().unwrap().size() != size;
    if update {
        *buffer = Some(ctx.device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        }));
    }
    update
}

/// returns true if buffer is updated
fn prepare_buffer_init_with_image(
    ctx: &Context,
    buffer: &mut Option<wgpu::Buffer>,
    image: &ImageBuffer<Luma<f32>, Vec<f32>>,
    usage: wgpu::BufferUsages,
) -> bool {
    let update = buffer.is_none()
        || buffer.as_ref().unwrap().size() != (image.as_raw().len() * size_of::<f32>()) as u64;
    if update {
        *buffer = Some(
            ctx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&image.as_raw()),
                    usage,
                }),
        );
    } else {
        ctx.queue.write_buffer(
            buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&image.as_raw()),
        );
    }
    update
}

#[cfg(test)]
mod tests {
    use crate::utils::save_luma32f;

    use super::*;
    use std::{error::Error, fs, path::PathBuf, time::Instant};

    // #[test]
    // fn foo() -> Result<(), Box<dyn Error>> {
    //     let image = image::open("./assets/in_battle.png")?;
    //     let template = image::open("./assets/battle_deploy-card-cost1.png")?;
    //     fs::create_dir_all("./assets/output")?;

    //     let image = image.to_luma32f();
    //     save_luma32f(&image, "./assets/output/grey.png", false);
    //     let image = ImageBuffer::from_fn(image.width(), image.height(), |x, y| {
    //         let mut sum = 0.0;
    //         let mut cnt = 0;
    //         for i in x..(x + template.width()).min(image.width()) {
    //             for j in y..(y + template.height()).min(image.height()) {
    //                 sum += image.get_pixel(i, j).0[0];
    //                 cnt += 1;
    //             }
    //         }
    //         // println!("{sum}/{cnt}");
    //         // println!("{} {}", image.get_pixel(x, y).0[0], sum / cnt as f32);
    //         Luma([(sum / cnt as f32)])
    //     });
    //     save_luma32f(&image, "./assets/output/avg.png", false);
    //     Ok(())
    // }

    fn init_profiling() {
        #[cfg(feature = "profiling")]
        {
            let _cpu_server =
                puffin_http::Server::new(&format!("0.0.0.0:{}", puffin_http::DEFAULT_PORT))
                    .unwrap();
            let _gpu_server = puffin_http::Server::new_custom(
                &format!("0.0.0.0:{}", puffin_http::DEFAULT_PORT + 1),
                |sink| super::PUFFIN_GPU_PROFILER.lock().unwrap().add_sink(sink),
                |id| {
                    _ = super::PUFFIN_GPU_PROFILER.lock().unwrap().remove_sink(id);
                },
            )
            .unwrap();
            Box::leak(Box::new(_cpu_server));
            Box::leak(Box::new(_gpu_server));
            puffin::set_scopes_on(true);
        }
    }

    #[test]
    fn foo() -> Result<(), Box<dyn Error>> {
        let angel= image::open("./assets/avatars/angel_sale#8.png")?.to_luma32f();
        let kalts = image::open("./assets/avatars/kalts.png")?.to_luma32f();

        let res = match_template(
            &angel,
            &kalts,
            MatchTemplateMethod::CrossCorrelation,
            false,
        );
        println!("{:?}", res.get_pixel(0, 0));
        let res = match_template(
            &kalts,
            &kalts,
            MatchTemplateMethod::CrossCorrelation,
            false,
        );
        println!("{:?}", res.get_pixel(0, 0));

        let image = image::open("./assets/in_battle.png")?.to_luma32f();
        let res = match_template(
            &image,
            &angel,
            MatchTemplateMethod::CrossCorrelation,
            false,
        );
        save_luma32f(&res, "./assets/output/foo.png", false);
        let res = find_extremes(&res);
        println!("{:?}", res);
        Ok(())
    }

    #[test]
    fn test_template_matching() -> Result<(), Box<dyn Error>> {
        init_profiling();

        let images = ["in_battle", "1-4_deploying", "1-4_deploying_direction"].map(|name| {
            (
                name,
                image::open(format!("./assets/{name}.png"))
                    .unwrap()
                    .to_luma32f(),
            )
        });
        let templates = ["battle_deploy-card-cost1", "battle_pause"].map(|name| {
            (
                name,
                image::open(format!("./assets/{name}.png"))
                    .unwrap()
                    .to_luma32f(),
            )
        });

        for template in templates {
            test_matching_all_methods(&template, &images)?;
        }
        Ok(())
    }

    fn test_matching_all_methods(
        template: &(&str, ImageBuffer<Luma<f32>, Vec<f32>>),
        images: &[(&str, ImageBuffer<Luma<f32>, Vec<f32>>)],
    ) -> Result<(), Box<dyn Error>> {
        let (template_name, template) = template;
        let dir = PathBuf::from(format!("./assets/output/{template_name}"));
        std::fs::create_dir_all(&dir)?;

        for method in [
            MatchTemplateMethod::SumOfSquaredDifference,
            MatchTemplateMethod::SumOfSquaredDifferenceNormed,
            MatchTemplateMethod::CrossCorrelation,
            MatchTemplateMethod::CrossCorrelationNormed,
            MatchTemplateMethod::CorrelationCoefficient,
            MatchTemplateMethod::CorrelationCoefficientNormed,
        ] {
            let method_dir = dir.join(format!("{}", method));
            fs::create_dir_all(&method_dir)?;

            for (name, image) in images.iter() {
                println!("matching using {}...", method);
                let t = Instant::now();
                let res = match_template(&image, &template, method, false);
                println!("cost: {:?}", t.elapsed());
                save_luma32f(
                    &res,
                    method_dir.join(format!("{name}-ap_cv.png")),
                    matches!(
                        method,
                        MatchTemplateMethod::SumOfSquaredDifference
                            | MatchTemplateMethod::CrossCorrelation
                            | MatchTemplateMethod::CorrelationCoefficient
                    ),
                );
            }
        }
        Ok(())
    }
}
