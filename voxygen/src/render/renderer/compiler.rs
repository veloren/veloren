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
    fn create_shader_module(
        &mut self,
        device: &wgpu::Device,
        source: &str,
        stage: ShaderStage,
        name: &str,
    ) -> Result<wgpu::ShaderModule, RenderError>;
}

pub(super) struct ShaderCCompiler {
    compiler: shaderc::Compiler,
    options: shaderc::CompileOptions<'static>,
}

impl ShaderCCompiler {
    pub(super) fn new(
        optimize: bool,
        resolve_include: impl Fn(&str, &str) -> Result<String, String> + 'static,
    ) -> Result<Self, RenderError> {
        let compiler = shaderc::Compiler::new()?;
        let mut options = shaderc::CompileOptions::new()?;

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
    fn create_shader_module(
        &mut self,
        device: &wgpu::Device,
        source: &str,
        stage: ShaderStage,
        name: &str,
    ) -> Result<wgpu::ShaderModule, RenderError> {
        prof_span!(_guard, "create_shader_modules");
        use std::borrow::Cow;

        let file_name = format!("{}.glsl", name);
        let file_name = file_name.as_str();

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

        let descriptor = wgpu::ShaderModuleDescriptor {
            label: Some(file_name),
            source: wgpu::ShaderSource::SpirV(Cow::Borrowed(spv.as_binary())),
        };
        let runtimechecks = wgpu::ShaderRuntimeChecks::unchecked();
        #[expect(unsafe_code)]
        Ok(unsafe { device.create_shader_module_trusted(descriptor, runtimechecks) })
    }
}

pub(super) struct WgpuCompiler {
    reg: regex::Regex,
    resolve_include: Box<dyn Fn(&str, &str) -> Result<String, String> + 'static>,
}

impl WgpuCompiler {
    pub(super) fn new(
        resolve_include: impl Fn(&str, &str) -> Result<String, String> + 'static,
    ) -> Result<Self, RenderError> {
        let reg = regex::Regex::new("(?mR)^#include +<(.+)>$").unwrap();
        Ok(Self {
            reg,
            resolve_include: Box::new(resolve_include),
        })
    }
}

impl Compiler for WgpuCompiler {
    fn create_shader_module(
        &mut self,
        device: &wgpu::Device,
        source: &str,
        stage: ShaderStage,
        name: &str,
    ) -> Result<wgpu::ShaderModule, RenderError> {
        use std::borrow::Cow;

        prof_span!(_guard, "create_shader_modules");

        let label = name;

        device.push_error_scope(wgpu::ErrorFilter::Validation);

        // replace all `includes` recursivly
        let mut source = Cow::Borrowed(source);
        let source = loop {
            let resolve_includes = self.reg.replace_all(&source, |cap: &regex::Captures| {
                (self.resolve_include)(cap.get(1).unwrap().as_str(), name).unwrap() //TODO unwrap! replace with https://docs.rs/regex/latest/regex/struct.Regex.html#fallibility
            });

            match resolve_includes {
                Cow::Borrowed(source) => break source,
                Cow::Owned(s) => source = Cow::Owned(s),
            }
        };

        let descriptor = wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Glsl {
                shader: Cow::Borrowed(source),
                stage: stage.into(),
                defines: &[],
            },
        };
        let runtimechecks = wgpu::ShaderRuntimeChecks::unchecked();
        #[expect(unsafe_code)]
        let shader = unsafe { device.create_shader_module_trusted(descriptor, runtimechecks) };

        let rt = tokio::runtime::Runtime::new().unwrap();

        if let Some(error) = rt.block_on(device.pop_error_scope()) {
            Err(RenderError::ShaderWgpuError(label.to_owned(), error))
        } else {
            Ok(shader)
        }
    }
}
