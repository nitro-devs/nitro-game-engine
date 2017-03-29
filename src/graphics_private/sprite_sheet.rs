use sdl2::render::Texture as SdlTexture;
use std::sync::Arc;
use math::IntRect;

/// An animation given by a SpriteSheet.
///
/// Animation is not done automatically by Nitro, advancing through and selecting
/// frames is typically the responsiblity of a Component delegated to controlling
/// animation.
pub struct SpriteSheet {
    pub animations: Vec<Vec<SpriteSheetFrame>>,
    texture: Arc<SdlTexture>,
    pub current_animation: u32,
    pub current_frame: u32,
}

pub struct SpriteSheetFrame {
    pub frame_rect: IntRect,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
}

pub fn get_texture(sprite_sheet: &SpriteSheet) -> &Arc<SdlTexture> {
    &sprite_sheet.texture
}
