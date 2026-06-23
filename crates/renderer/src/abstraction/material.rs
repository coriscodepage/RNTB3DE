use crate::abstraction::{handles::TextureHandle, program::Program};

pub struct Material<T, VS, FS> {
    program: Program<T, VS, FS>,
    textures: [TextureHandle; 8],
}