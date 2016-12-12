use update_component::UpdateComponent;
use game_object::GameObject;

pub struct Spinny {

}

impl UpdateComponent for Spinny {
    fn update(&mut self, game_object : &mut GameObject, delta_time : f64) {
        game_object.rot += 1.0 * delta_time;
    }
}