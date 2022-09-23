use std::{
    collections::HashMap,
    fs::File,
    io::{prelude::*, SeekFrom},
};
type Result = std::io::Result<()>;

use common::{
    terrain::{Block, BlockKind, SpriteKind},
    vol::{BaseVol, ReadVol, RectSizedVol, WriteVol},
};
use rayon::ThreadPoolBuilder;
use vek::{Vec2, Vec3};
use veloren_world::{
    sim::{FileOpts, WorldOpts, DEFAULT_WORLD_MAP},
    site2::{plot::PlotKind, Fill, Structure},
    CanvasInfo, Land, World,
};

/// This exports a dungeon (structure only, no entities or sprites) to a
/// MagicaVoxel .vox file

fn main() -> Result {
    common_frontend::init_stdout(None);
    let pool = ThreadPoolBuilder::new().build().unwrap();
    println!("Loading world");
    let (world, index) = World::generate(
        59686,
        WorldOpts {
            seed_elements: true,
            world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
            calendar: None,
        },
        &pool,
    );
    println!("Loaded world");
    let export_path = "dungeon.vox";

    println!("Saving into {}", export_path);
    let mut volume = ExportVol::new();
    let wpos = volume.size_xy().map(|p| p as i32 / 2);
    let site =
        veloren_world::site2::Site::generate_dungeon(&Land::empty(), &mut rand::thread_rng(), wpos);
    CanvasInfo::with_mock_canvas_info(index.as_index_ref(), world.sim(), |canvas| {
        for plot in site.plots() {
            if let PlotKind::Dungeon(dungeon) = plot.kind() {
                let (prim_tree, fills, _entities) = dungeon.render_collect(&site, canvas);

                for (prim, fill) in fills {
                    let aabb = Fill::get_bounds(&prim_tree, prim);

                    for x in aabb.min.x..aabb.max.x {
                        for y in aabb.min.y..aabb.max.y {
                            for z in aabb.min.z..aabb.max.z {
                                let pos = Vec3::new(x, y, z);

                                let _ = volume.map(pos, |block| {
                                    if let Some(block) =
                                        fill.sample_at(&prim_tree, prim, pos, canvas, block)
                                    {
                                        block
                                    } else {
                                        block
                                    }
                                });
                            }
                        }
                    }
                }
            }
        }
    });

    volume.write(&mut File::create(export_path)?)
}

struct ExportVol {
    models: HashMap<Vec3<i32>, Vec<u8>>,
    width: i32,
    default_block: Block,
}

impl ExportVol {
    const CHUNK_SIZE: i32 = 256;

    fn new() -> Self {
        Self {
            models: HashMap::new(),
            width: 1000,
            default_block: Block::empty(),
        }
    }

    fn write(&self, file: &mut File) -> Result {
        // We need to split the structure into multiple models if it's too big
        // However, the create_vox crate doesn't yet work with scene graphs
        // Luckily, writing vox files is easy enough
        // File format defined at https://github.com/ephtracy/voxel-model

        fn write_i32(file: &mut File, value: i32) -> Result {
            // The spec doesn't specify endianness?!?
            file.write_all(&value.to_le_bytes())
        }

        fn write_chunk(
            file: &mut File,
            name: &str,
            write_body: &dyn Fn(&mut File) -> Result,
        ) -> Result {
            file.write_all(name.as_bytes())?;
            write_i32(file, 28)?; // Chunk size (unknown at this point)
            write_i32(file, 0)?; // Size of child chunks
            let chunk_start = file.stream_position()?;
            write_body(file)?;
            let chunk_end = file.stream_position()?;
            file.seek(SeekFrom::Start(chunk_start - 8))?;
            write_i32(file, chunk_end as i32 - chunk_start as i32)?;
            file.seek(SeekFrom::Start(chunk_end))?;
            Ok(())
        }

        fn write_translation_node(
            file: &mut File,
            id: i32,
            child_id: i32,
            pos: Vec3<i32>,
        ) -> Result {
            write_chunk(file, "nTRN", &|file| {
                write_i32(file, id)?; // Node index
                write_i32(file, 0)?; // Number of attributes
                write_i32(file, child_id)?; // Child node index
                write_i32(file, -1)?; // Reserved
                write_i32(file, 0)?; // Layer
                write_i32(file, 1)?; // Frames
                write_i32(file, 1)?; // Number of frame attributes
                write_i32(file, "_t".len() as i32)?; // Attribute name len
                file.write_all("_t".as_bytes())?; // Attribute name
                let translation_string = format!("{} {} {}", pos.x, pos.y, pos.z);
                write_i32(file, translation_string.len() as i32)?; // Value len
                file.write_all(translation_string.as_bytes()) // Value
            })
        }

        write!(file, "VOX ")?; // Magic number
        write_i32(file, 150)?; // Version

        write!(file, "MAIN")?;
        write_i32(file, 0)?; // Chunk size
        write_i32(file, 0)?; // Size of child chunks (set later)
        let chunks_start = file.stream_position()?;

        // Model data
        for (_, model) in self.models.iter() {
            write_chunk(file, "SIZE", &|file| {
                write_i32(file, Self::CHUNK_SIZE)?; // Size X
                write_i32(file, Self::CHUNK_SIZE)?; // Size Y
                write_i32(file, Self::CHUNK_SIZE) // Size Z
            })?;
            write_chunk(file, "XYZI", &|file| {
                write_i32(file, model.len() as i32 / 4)?; // Number of voxels
                file.write_all(model)
            })?;
        }

        // Scene graph
        // Root Transform node
        write_translation_node(file, 0, 1, Vec3::new(0, 0, 0))?;

        // Group node
        write_chunk(file, "nGRP", &|file| {
            write_i32(file, 1)?; // Node index
            write_i32(file, 0)?; // Number of attributes
            write_i32(file, self.models.len() as i32)?; // Number of child nodes
            for index in 0..self.models.len() {
                write_i32(file, index as i32 * 2 + 2)?;
            }
            Ok(())
        })?;

        for (index, (model_pos, _)) in self.models.iter().enumerate() {
            // Transform node
            let pos =
                model_pos.map(|p| p * Self::CHUNK_SIZE + Self::CHUNK_SIZE / 2 + i32::from(p >= 0));
            let pos = pos - Vec3::new(self.width / 2, self.width / 2, 0);
            let transform_node_id = index as i32 * 2 + 2;
            let shape_node_id = index as i32 * 2 + 3;
            write_translation_node(file, transform_node_id, shape_node_id, pos)?;

            // Shape node
            write_chunk(file, "nSHP", &|file| {
                write_i32(file, shape_node_id)?;
                write_i32(file, 0)?; // Number of attributes
                write_i32(file, 1)?; // Number of models
                write_i32(file, index as i32)?; // Model index (independent of scene graph index)
                write_i32(file, 0) // Number model of attributes
            })?;
        }

        // Palette
        write_chunk(file, "RGBA", &|file| {
            file.write_all(&[220, 220, 255, 0])?; // Air
            file.write_all(&[100, 100, 100, 0])?; // Rock
            file.write_all(&[255, 0, 0, 0])?; // Sprite
            file.write_all(&[255, 0, 255, 0])?; // GlowingRock
            file.write_all(&[0; 4 * (256 - 4)])
        })?;

        let chunks_end = file.stream_position()?;
        file.seek(SeekFrom::Start(chunks_start - 4))?;
        write_i32(file, chunks_end as i32 - chunks_start as i32)?;

        Ok(())
    }
}

impl BaseVol for ExportVol {
    type Error = ();
    type Vox = Block;
}

impl RectSizedVol for ExportVol {
    fn lower_bound_xy(&self) -> Vec2<i32> { Vec2::new(0, 0) }

    fn upper_bound_xy(&self) -> Vec2<i32> { Vec2::new(self.width, self.width) }
}

impl ReadVol for ExportVol {
    fn get(&self, _: Vec3<i32>) -> std::result::Result<&Self::Vox, Self::Error> {
        Ok(&self.default_block)
    }
}

impl WriteVol for ExportVol {
    fn set(
        &mut self,
        pos: Vec3<i32>,
        vox: Self::Vox,
    ) -> std::result::Result<Self::Vox, Self::Error> {
        // Because the dungeon may need to be split into multiple models, we can't
        // stream this directly to the file

        let model_pos = pos.map(|p| p.div_euclid(Self::CHUNK_SIZE));
        let rel_pos = pos.map(|p| (p % Self::CHUNK_SIZE) as u8);
        self.models
            .entry(model_pos)
            .or_default()
            .extend_from_slice(&[rel_pos.x, rel_pos.y, rel_pos.z, match vox.kind() {
                BlockKind::Air => {
                    if !matches!(vox.get_sprite(), Some(SpriteKind::Empty)) {
                        3
                    } else {
                        1
                    }
                },
                BlockKind::Rock => 2,
                BlockKind::GlowingRock => 4,
                _ => 5,
            }]);
        Ok(vox)
    }
}
