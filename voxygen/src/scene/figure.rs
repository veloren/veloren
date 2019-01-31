// Crate
use crate::{
    Error,
    render::{
        Consts,
        Globals,
        Mesh,
        Model,
        Renderer,
        FigurePipeline,
        FigureBoneData,
        FigureLocals,
    },
    anim::Skeleton,
};

pub struct Figure<S: Skeleton> {
    // GPU data
    model: Model<FigurePipeline>,
    bone_consts: Consts<FigureBoneData>,
    locals: Consts<FigureLocals>,

    // CPU data
    bone_meshes: [Option<Mesh<FigurePipeline>>; 16],
    pub skeleton: S,
}

impl<S: Skeleton> Figure<S> {
    pub fn new(
        renderer: &mut Renderer,
        bone_meshes: [Option<Mesh<FigurePipeline>>; 16],
        skeleton: S,
    ) -> Result<Self, Error> {
        let mut this = Self {
            model: renderer.create_model(&Mesh::new())?,
            bone_consts: renderer.create_consts(&skeleton.compute_matrices())?,
            locals: renderer.create_consts(&[FigureLocals::default()])?,

            bone_meshes,
            skeleton,
        };
        this.update_model(renderer)?;
        Ok(this)
    }

    pub fn update_model(&mut self, renderer: &mut Renderer) -> Result<(), Error> {
        let mut mesh = Mesh::new();

        self.bone_meshes
            .iter()
            .enumerate()
            .filter_map(|(i, bm)| bm.as_ref().map(|bm| (i, bm)))
            .for_each(|(i, bone_mesh)| {
                mesh.push_mesh_map(bone_mesh, |vert| vert.with_bone_idx(i as u8))
            });

        self.model = renderer.create_model(&mesh)?;
        Ok(())
    }

    pub fn update_skeleton(&mut self, renderer: &mut Renderer) -> Result<(), Error> {
        renderer.update_consts(&mut self.bone_consts, &self.skeleton.compute_matrices())?;
        Ok(())
    }

    pub fn update_locals(&mut self, renderer: &mut Renderer, locals: FigureLocals) -> Result<(), Error> {
        renderer.update_consts(&mut self.locals, &[locals])?;
        Ok(())
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        renderer.render_figure(
            &self.model,
            globals,
            &self.locals,
            &self.bone_consts,
        );
    }
}
