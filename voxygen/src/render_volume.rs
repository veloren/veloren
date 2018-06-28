use nalgebra::Vector4;

use region::{Voxel, Volume, Block, BlockMaterial, Chunk, Cell, Model};

pub trait RenderVoxel: Voxel {
    fn get_color(&self) -> Vector4<f32>;
    fn is_opaque(&self) -> bool;
}

pub trait RenderVolume: Volume
    where Self::VoxelType: RenderVoxel
{}

// Implementations for common structures

impl RenderVoxel for Block {
    fn get_color(&self) -> Vector4<f32> {
        let color_map = enum_map! {
            BlockMaterial::Air => Vector4::new(0.0, 0.0, 0.0, 0.0),
            BlockMaterial::Grass => Vector4::new(0.2, 0.7, 0.0, 1.0),
            BlockMaterial::Sand  => Vector4::new(1.0, 0.8, 0.5, 1.0),
            BlockMaterial::Earth => Vector4::new(0.5, 0.2, 0.0, 1.0),
            BlockMaterial::Stone => Vector4::new(0.5, 0.5, 0.5, 1.0),
            BlockMaterial::Water => Vector4::new(0.5, 0.7, 1.0, 1.0),
        };

        color_map[self.material()]
    }

    fn is_opaque(&self) -> bool {
        self.material() != BlockMaterial::Air
    }
}

impl RenderVolume for Chunk {}

impl RenderVoxel for Cell {
    fn get_color(&self) -> Vector4<f32> {
        let color_map = enum_map! {
            0 => Vector4::new(0.77, 0.11, 0.0, 1.0),
            1 => Vector4::new(1.0, 0.14, 0.0, 1.0),
            2 => Vector4::new(0.65, 0.22, 0.22, 1.0),
            3 => Vector4::new(0.55, 0.13, 0.13, 1.0),
            4 => Vector4::new(0.36, 0.07, 0.07, 1.0),
            5 => Vector4::new(0.99, 0.21, 0.21, 1.0),
            6 => Vector4::new(0.83, 1.0, 0.19, 1.0),
            7 => Vector4::new(1.0, 1.0, 1.0, 1.0),
            8 => Vector4::new(0.43, 0.0, 0.0, 1.0),
            9 => Vector4::new(1.0, 0.66, 0.0, 1.0),
            10 => Vector4::new(0.63, 0.42, 0.12, 1.0),
            11 => Vector4::new(0.31, 0.12, 0.12, 1.0),
            12 => Vector4::new(1.0, 0.34, 0.0, 1.0),
            13 => Vector4::new(0.78, 0.0, 0.01, 1.0),
            14 => Vector4::new(1.0, 0.31, 0.32, 1.0),
            15 => Vector4::new(1.0, 0.0, 0.0, 1.0),
            16 => Vector4::new(0.09, 0.36, 0.47, 1.0),
            17 => Vector4::new(0.52, 0.91, 1.0, 1.0),
            18 => Vector4::new(0.0, 0.88, 1.0, 1.0),
            19 => Vector4::new(0.39, 0.62, 1.0, 1.0),
            20 => Vector4::new(0.22, 0.20, 0.64, 1.0),
            21 => Vector4::new(0.15, 0.15, 0.38, 1.0),
            22 => Vector4::new(0.01, 0.02, 0.47, 1.0),
            23 => Vector4::new(0.0, 0.02, 0.29, 1.0),
            24 => Vector4::new(0.25, 0.19, 1.0, 1.0),
            25 => Vector4::new(0.37, 0.32, 1.0, 1.0),
            26 => Vector4::new(0.0, 0.73, 1.0, 1.0),
            27 => Vector4::new(0.27, 0.37, 1.0, 1.0),
            28 => Vector4::new(0.13, 0.23, 0.67, 1.0),
            29 => Vector4::new(0.26, 0.0, 1.0, 1.0),
            30 => Vector4::new(0.0, 0.58, 0.30, 1.0),
            31 => Vector4::new(0.17, 1.0, 0.56, 1.0),
            32 => Vector4::new(0.59, 0.41, 0.20, 1.0),
            33 => Vector4::new(0.35, 0.08, 0.08, 1.0),
            34 => Vector4::new(0.58, 0.34, 0.0, 1.0),
            35 => Vector4::new(0.45, 0.29, 0.0, 1.0),
            36 => Vector4::new(0.36, 0.20, 0.0, 1.0),
            37 => Vector4::new(0.33, 0.13, 0.0, 1.0),
            38 => Vector4::new(0.22, 0.16, 0.04, 1.0),
            39 => Vector4::new(0.25, 0.11, 0.0, 1.0),
            40 => Vector4::new(0.52, 0.35, 0.1, 1.0),
            41 => Vector4::new(1.0, 0.44, 0.0, 1.0),
            42 => Vector4::new(1.0, 0.61, 0.44, 1.0),
            43 => Vector4::new(0.34, 0.17, 0.0, 1.0),
            44 => Vector4::new(0.49, 0.49, 0.09, 1.0),
            45 => Vector4::new(0.42, 0.31, 0.07, 1.0),
            46 => Vector4::new(0.54, 0.36, 0.09, 1.0),
            47 => Vector4::new(0.40, 0.36, 0.21, 1.0),
            48 => Vector4::new(0.28, 0.58, 0.23, 1.0),
            49 => Vector4::new(0.14, 0.48, 0.15, 1.0),
            50 => Vector4::new(0.14, 0.48, 0.15, 1.0),
            51 => Vector4::new(0.0, 0.78, 0.0, 1.0),
            52 => Vector4::new(0.0, 0.64, 0.01, 1.0),
            53 => Vector4::new(0.0, 0.52, 0.11, 1.0),
            54 => Vector4::new(0.0, 0.37, 0.07, 1.0),
            55 => Vector4::new(0.0, 0.21, 0.2, 1.0),
            56 => Vector4::new(0.36, 0.42, 0.0, 1.0),
            57 => Vector4::new(0.47, 0.8, 0.4, 1.0),
            58 => Vector4::new(0.32, 0.76, 0.0, 1.0),
            59 => Vector4::new(0.45, 0.83, 0.0, 1.0),
            60 => Vector4::new(0.33, 1.0, 0.0, 1.0),
            61 => Vector4::new(0.52, 0.7, 0.0, 1.0),
            62 => Vector4::new(0.48, 0.66, 0.16, 1.0),
            63 => Vector4::new(0.21, 0.87, 0.29, 1.0),
            64 => Vector4::new(1.0, 0.72, 0.38, 1.0),
            65 => Vector4::new(0.69, 0.69, 0.31, 1.0),
            66 => Vector4::new(0.96, 1.0, 0.4, 1.0),
            67 => Vector4::new(1.0, 1.0, 0.14, 1.0),
            68 => Vector4::new(1.0, 0.89, 0.0, 1.0),
            69 => Vector4::new(0.8, 0.67, 0.0, 1.0),
            70 => Vector4::new(0.63, 0.63, 0.0, 1.0),
            71 => Vector4::new(0.54, 0.66, 0.0, 1.0),
            72 => Vector4::new(1.0, 0.84, 0.0, 1.0),
            73 => Vector4::new(0.95, 1.0, 0.2, 1.0),
            74 => Vector4::new(1.0, 0.76, 0.41, 1.0),
            75 => Vector4::new(1.0, 0.88, 0.31, 1.0),
            76 => Vector4::new(1.0, 0.75, 0.19, 1.0),
            77 => Vector4::new(1.0, 0.93, 0.36, 1.0),
            78 => Vector4::new(1.0, 0.62, 0.24, 1.0),
            79 => Vector4::new(1.0, 0.51, 0.19, 1.0),
            80 => Vector4::new(0.49, 0.38, 0.51, 1.0),
            81 => Vector4::new(0.54, 0.28, 0.56, 1.0),
            82 => Vector4::new(0.72, 0.18, 0.74, 1.0),
            83 => Vector4::new(0.61, 0.0, 0.92, 1.0),
            84 => Vector4::new(0.6, 0.6, 1.0, 1.0),
            85 => Vector4::new(0.6, 0.6, 0.8, 1.0),
            86 => Vector4::new(0.6, 0.6, 0.6, 1.0),
            87 => Vector4::new(0.6, 0.6, 0.4, 1.0),
            88 => Vector4::new(0.6, 0.19, 0.4, 1.0),
            89 => Vector4::new(0.77, 0.24, 0.59, 1.0),
            90 => Vector4::new(1.0, 0.45, 0.73, 1.0),
            91 => Vector4::new(0.52, 0.24, 0.8, 1.0),
            92 => Vector4::new(0.6, 0.06, 0.6, 1.0),
            93 => Vector4::new(0.47, 0.39, 0.6, 1.0),
            94 => Vector4::new(0.54, 0.4, 0.6, 1.0),
            95 => Vector4::new(0.6, 0.21, 0.5, 1.0),
            96 => Vector4::new(0.42, 0.42, 0.42, 1.0),
            97 => Vector4::new(0.83, 0.0, 1.0, 1.0),
            98 => Vector4::new(0.11, 0.0, 1.0, 1.0),
            99 => Vector4::new(0.0, 0.98, 1.0, 1.0),
            100 => Vector4::new(0.08, 1.0, 0.0, 1.0),
            101 => Vector4::new(1.0, 0.8, 0.0, 1.0),
            102 => Vector4::new(1.0, 0.33, 0.0, 1.0),
            103 => Vector4::new(1.0, 0.0, 0.01, 1.0),
            104 => Vector4::new(0.14, 0.14, 0.14, 1.0),
            105 => Vector4::new(0.89, 0.42, 1.0, 1.0),
            106 => Vector4::new(0.06, 0.0, 0.6, 1.0),
            107 => Vector4::new(0.0, 0.49, 0.51, 1.0),
            108 => Vector4::new(0.24, 0.6, 0.21, 1.0),
            109 => Vector4::new(0.66, 0.52, 0.0, 1.0),
            110 => Vector4::new(1.0, 0.78, 0.39, 1.0),
            111 => Vector4::new(1.0, 0.39, 0.40, 1.0),
            112 => Vector4::new(0.52, 0.52, 0.52, 1.0),
            113 => Vector4::new(0.25, 0.53, 0.7, 1.0),
            114 => Vector4::new(0.45, 0.61, 0.70, 1.0),
            115 => Vector4::new(0.25, 0.50, 0.50, 1.0),
            116 => Vector4::new(0.27, 0.40, 0.33, 1.0),
            117 => Vector4::new(0.28, 0.28, 0.28, 1.0),
            118 => Vector4::new(0.2, 0.2, 0.2, 1.0),
            119 => Vector4::new(0.1, 0.1, 0.1, 1.0),
            120 => Vector4::new(0.35, 0.35, 0.47, 1.0),
            121 => Vector4::new(0.31, 0.31, 0.47, 1.0),
            122 => Vector4::new(0.24, 0.24, 0.47, 1.0),
            123 => Vector4::new(0.16, 0.16, 0.47, 1.0),
            124 => Vector4::new(0.09, 0.1, 0.47, 1.0),
            125 => Vector4::new(0.0, 0.01, 0.36, 1.0),
            126 => Vector4::new(0.83, 0.83, 0.83, 1.0),
            127 => Vector4::new(0.67, 0.67, 0.67, 1.0),
            128 => Vector4::new(0.67, 0.67, 0.67, 1.0),
            129 => Vector4::new(0.62, 0.62, 0.62, 1.0),
            130 => Vector4::new(0.55, 0.55, 0.55, 1.0),
            131 => Vector4::new(0.47, 0.47, 0.47, 1.0),
            132 => Vector4::new(0.39, 0.39, 0.39, 1.0),
            133 => Vector4::new(0.28, 0.28, 0.28, 1.0),
            134 => Vector4::new(0.2, 0.2, 0.2, 1.0),
            135 => Vector4::new(0.1, 0.1, 0.1, 1.0),
            136 => Vector4::new(0.35, 0.2, 0.4, 1.0),
            137 => Vector4::new(0.52, 0.35, 0.48, 1.0),
            138 => Vector4::new(0.59, 0.4, 0.55, 1.0),
            139 => Vector4::new(0.69, 0.47, 0.64, 1.0),
            140 => Vector4::new(0.85, 0.59, 0.8, 1.0),
            141 => Vector4::new(0.85, 0.85, 0.85, 1.0),
            142 => Vector4::new(0.83, 0.83, 0.83, 1.0),
            143 => Vector4::new(0.67, 0.67, 0.67, 1.0),
            144 => Vector4::new(0.23, 0.23, 0.23, 1.0),
            145 => Vector4::new(0.35, 0.35, 0.35, 1.0),
            146 => Vector4::new(0.41, 0.41, 0.41, 1.0),
            147 => Vector4::new(0.65, 0.65, 0.65, 1.0),
            148 => Vector4::new(0.0, 0.0, 0.0, 1.0),
            255 => Vector4::new(0.0, 0.0, 0.0, 0.0),
            //51 => Vector4::new(0., 0., 0., 1.0),
            _ => Vector4::new(0.0, 0.0, 0.0, 1.0),
        };

        color_map[self.material()]
    }

    fn is_opaque(&self) -> bool {
        self.material() != 255
    }
}

impl RenderVolume for Model {}
