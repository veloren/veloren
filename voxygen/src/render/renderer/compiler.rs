use common_base::prof_span;
use tracing::info;

use crate::render::RenderError;

pub(super) enum ShaderStage {
    Vertex,
    Fragment,
}

impl From<ShaderStage> for wgpu::naga::ShaderStage {
    fn from(value: ShaderStage) -> Self {
        match value {
            ShaderStage::Vertex => wgpu::naga::ShaderStage::Vertex,
            ShaderStage::Fragment => wgpu::naga::ShaderStage::Fragment,
        }
    }
}

impl From<ShaderStage> for shaderc::ShaderKind {
    fn from(value: ShaderStage) -> Self {
        match value {
            ShaderStage::Vertex => shaderc::ShaderKind::Vertex,
            ShaderStage::Fragment => shaderc::ShaderKind::Fragment,
        }
    }
}

pub(super) trait Compiler {
    type ShaderInfo;

    fn create_shader_module(
        &mut self,
        device: &wgpu::Device,
        source: &str,
        stage: ShaderStage,

        info: Self::ShaderInfo,
    ) -> Result<wgpu::ShaderModule, RenderError>;
}

pub(super) struct ShaderCCompiler {
    compiler: shaderc::Compiler,
    options: shaderc::CompileOptions<'static>,
}

pub(super) struct ShaderCInfo {
    pub file_name: String,
}

impl ShaderCCompiler {
    pub(super) fn new(
        optimize: bool,
        resolve_include: impl Fn(&str, &str) -> Result<String, String> + 'static,
    ) -> Result<Self, RenderError> {
        let compiler = shaderc::Compiler::new().ok_or(RenderError::ErrorInitializingCompiler)?;
        let mut options =
            shaderc::CompileOptions::new().ok_or(RenderError::ErrorInitializingCompiler)?;

        if optimize {
            options.set_optimization_level(shaderc::OptimizationLevel::Performance);
            info!("Enabled optimization by shaderc.");
        } else {
            options.set_optimization_level(shaderc::OptimizationLevel::Zero);
            info!("Disabled optimization by shaderc.");
        }
        options.set_forced_version_profile(430, shaderc::GlslProfile::Core);
        // options.set_generate_debug_info();
        options.set_include_callback(move |name, _, shader_name, _| {
            Ok(shaderc::ResolvedInclude {
                resolved_name: name.to_string(),
                content: resolve_include(name, shader_name)?,
            })
        });

        Ok(Self { compiler, options })
    }
}

impl Compiler for ShaderCCompiler {
    type ShaderInfo = ShaderCInfo;

    fn create_shader_module(
        &mut self,
        device: &wgpu::Device,
        source: &str,
        stage: ShaderStage,
        info: Self::ShaderInfo,
    ) -> Result<wgpu::ShaderModule, RenderError> {
        prof_span!(_guard, "create_shader_modules");
        use std::borrow::Cow;

        let file_name = info.file_name.as_str();

        let spv = self
            .compiler
            .compile_into_spirv(source, stage.into(), file_name, "main", Some(&self.options))
            .map_err(|e| (file_name, e))?;

        // Uncomment me to dump shaders to files
        //
        // std::fs::create_dir_all("dumpped-shaders").expect("Couldn't create shader
        // dumps folders");
        //
        // let mut file = std::fs::File::create(format!("dumpped-shaders/{}.spv",
        // file_name))     .expect("Couldn't open shader out");
        //
        // use std::io::Write;
        //
        // file.write(spv.as_binary_u8())
        //     .expect("Couldn't write shader out");

        // let label = [file_name, "\n\n", source].concat();
        #[expect(unsafe_code)]
        Ok(unsafe {
            device.create_shader_module_trusted(
                wgpu::ShaderModuleDescriptor {
                    label: Some(file_name),
                    source: wgpu::ShaderSource::SpirV(Cow::Borrowed(spv.as_binary())),
                },
                wgpu::ShaderRuntimeChecks::unchecked(),
            )
        })
    }
}
