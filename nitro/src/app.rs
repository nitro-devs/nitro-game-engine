use sdl2;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::render::Renderer;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::image::LoadTexture;
use audio_private;
use audio_private::dj;
use audio::Dj;
use input_private;
use input::Input;
use game_object::GameObject;
use game_object;
use component::Message;
use texture::Texture;
use texture;
use PolarCoords;
use Vector;
use transform::Transform;
use transform;
use camera::Camera;
use nphysics2d::world::World;
use rodio;
use rodio::Decoder;
use rodio::source::Buffered;
use rodio::Source;
use std::collections::HashMap;
use std::borrow::BorrowMut;
use std::fs::File;
use std::io::BufReader;
use std::cell::RefCell;
use std::path::PathBuf;
use std::f32;
use std::time::Instant;
use chrono::Duration;

type BufferedAudioFile = Buffered<Decoder<BufReader<File>>>;

pub struct App {
    renderer: Renderer<'static>,
    event_pump: EventPump,
    game_objects: HashMap<u64, Box<GameObject>>,
    djs: HashMap<u64, Box<Dj>>,
    next_game_object_id: u64,
    next_dj_id: u64,
    pub input: Input,
    sound_cache: HashMap<String, Box<BufferedAudioFile>>,
    pub camera: Camera,
    pub world: World<f32>,
}

impl App {
    pub fn new(name: &str) -> App {
        let sdl_context = sdl2::init().expect("Failed to initialize SDL2.");
        let video_subsystem = sdl_context.video().expect("Failed to initialize video subsystem");
        let window = video_subsystem.window("Halera", 800, 600)
            .position_centered()
            .fullscreen_desktop()
            .opengl()
            .build()
            .unwrap();
        let renderer = window.renderer().build().expect("Failed to initialize renderer");
        App {
            next_game_object_id: 0,
            next_dj_id: 0,
            game_objects: HashMap::new(),
            djs: HashMap::new(),
            input: Input::new(),
            sound_cache: HashMap::new(),
            renderer: renderer,
            event_pump: sdl_context.event_pump().expect("Failed to initalize event pump."),
            camera: Camera { transform: Transform::new() },
            world: World::new(),
        }
    }

    fn render(&mut self) {
        use std::f32;
        let game_objs = &self.game_objects;
        let camera_transform = self.camera.transform;
        // Clear the screen with grey.
        self.renderer.set_draw_color(Color::RGB(240, 240, 240));
        self.renderer.clear();
        // Reset the draw color to white so subsequent draw calls are correct.
        self.renderer.set_draw_color(Color::RGB(255, 255, 255));
        let (screen_width, screen_height) = self.renderer.window().unwrap().size();
        for game_obj in game_objs.values() {
            let mut render_transform = game_obj.transform;
            *render_transform.mut_x() -= *camera_transform.x();
            *render_transform.mut_y() -= *camera_transform.y();
            let mut polar = PolarCoords::from(render_transform.position().clone());
            polar.rotation -= *camera_transform.rotation();
            *render_transform.mut_position() = Vector::from(polar);
            *render_transform.mut_x() += (screen_width / 2) as f32;
            *render_transform.mut_y() += (screen_height / 2) as f32;
            *render_transform.mut_rotation() -= *camera_transform.rotation();
            let (tex_width, tex_height) = texture::size(&game_obj.texture);
            let render_rect = Rect::new(((*render_transform.x()) as i32) - (tex_width as i32 / 2),
                                        ((*render_transform.y()) as i32) - (tex_height as i32 / 2),
                                        tex_width,
                                        tex_height);
            if let &Some(ref texture) = texture::get_raw(&game_obj.texture) {
                self.renderer.copy_ex(&texture,
                                      None,
                                      Some(render_rect),
                                      (*game_obj.transform.rotation() * 180.0/f32::consts::PI) as f64,
                                      None,
                                      false,
                                      false);
            }
        }
        self.renderer.present();
    }

    fn update(&mut self, delta_time: f32) {
        //Process Djs for this frame.
        let keys = self.djs.keys().map(|x| *x).collect::<Vec<u64>>();
        for key in keys {
            if let Some(mut dj) = self.djs.remove(&key) {
                if dj.is_over() {
                    let (game_object_id, component_id) = dj::get_listener(&dj);
                    let message = Message::DjIdle { dj: RefCell::new(dj.borrow_mut()) };
                    if let Some(mut game_object) = self.game_objects.remove(&game_object_id) {
                        game_object.send_message_to_component(self, &message, component_id);
                        self.game_objects.insert(game_object.id(), game_object);
                    }
                }
                if !dj::was_dropped(&dj) {
                    self.djs.insert(dj.id(), dj);
                }
            }
        }
        //Copy game_object to the physics world, step, then copy from physics to game_object
        for mut game_object in self.game_objects.values_mut() {
            game_object::copy_to_physics(game_object);
        }
        self.world.step(delta_time);
        for mut game_object in self.game_objects.values_mut() {
            game_object::copy_from_physics(game_object);
        }

        //Send update messages
        let keys = self.game_objects.keys().map(|x| *x).collect::<Vec<u64>>();
        for key in keys {
            if let Some(mut game_object) = self.game_objects.remove(&key) {
                game_object.update(self, delta_time);
                *game_object.transform.mut_rotation() %= 2.0 * f32::consts::PI;
                if !game_object::was_dropped(game_object.borrow_mut()) {
                    self.game_objects.insert(game_object.id(), game_object);
                }
            }
        }
        input_private::input::shift_frame(&mut self.input);
    }

    pub fn run(&mut self) {
        let mut exit = false;
        let mut last_frame_instant = Instant::now();
        while !exit {
            while let Some(e) = self.event_pump.poll_event() {
                if let Event::Quit{..} = e {
                    exit = true;
                }
                input_private::input::process_event(&mut self.input, &e);
            }
            self.update(Duration::from_std(last_frame_instant.elapsed())
                .unwrap()
                .num_microseconds().unwrap() as f32 / 1000000.0);
            last_frame_instant = Instant::now();
            self.render();
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn play_sound(&mut self, path: &str, volume: f32) {
        let mut sink = rodio::Sink::new(&rodio::get_default_endpoint().unwrap());
        sink.set_volume(volume);
        sink.append(fetch_sound(self, path));
        sink.detach();
    }

    pub fn new_dj<F>(&mut self, f: F) -> u64
        where F: FnOnce(&mut App, &mut Dj)
    {
        let sink = rodio::Sink::new(&rodio::get_default_endpoint().unwrap());
        let mut dj = Box::new(audio_private::dj::new_dj(sink, self));
        f(self, dj.borrow_mut());
        let id = dj.id();
        self.djs.insert(id, dj);
        id
    }

    pub fn dj_by_id(&self, id: u64) -> Option<&Dj> {
        if let Some(boxxed) = self.djs.get(&id) {
            Some(boxxed.as_ref())
        } else {
            None
        }
    }

    pub fn dj_by_id_mut(&mut self, id: u64) -> Option<&mut Dj> {
        if let Some(boxxed) = self.djs.get_mut(&id) {
            Some(boxxed.borrow_mut())
        } else {
            None
        }
    }

    // Load a sound into the cache if it's not already there.
    pub fn load_sound(&mut self, path: &str) {
        if !self.sound_cache.contains_key(path) {
            let mut sound_path = PathBuf::from("assets");
            sound_path.push("sounds");
            sound_path.push(path);
            let file = File::open(sound_path).unwrap();
            let new_source =
                Box::new(rodio::Decoder::new(BufReader::new(file)).unwrap().buffered());
            self.sound_cache.insert(path.to_owned(), new_source);
        }
    }

    pub fn unload_sound(&mut self, path: &str) {
        self.sound_cache.remove(path);
    }

    pub fn new_gameobject<F>(&mut self, f: F) -> u64
        where F: FnOnce(&mut App, &mut GameObject)
    {
        let mut game_object = Box::new(game_object::new(self));
        f(self, &mut game_object);
        let id = game_object.id();
        self.game_objects.insert(id, game_object);
        id
    }

    pub fn game_object_by_id(&self, id: u64) -> Option<&GameObject> {
        if let Some(boxxed) = self.game_objects.get(&id) {
            Some(boxxed.as_ref())
        } else {
            None
        }
    }

    pub fn game_object_by_id_mut(&mut self, id: u64) -> Option<&mut GameObject> {
        if let Some(boxxed) = self.game_objects.get_mut(&id) {
            Some(boxxed.borrow_mut())
        } else {
            None
        }
    }

    pub fn fetch_texture(&mut self, texture_name: &str) -> Result<Texture, String> {
        let mut texture_path = PathBuf::from("assets");
        texture_path.push("textures");
        texture_path.push(texture_name);
        let sdl_texture = self.renderer.load_texture(texture_path)?;
        let mut nitro_texture = Texture::new();
        texture::set_raw(&mut nitro_texture, sdl_texture);
        Ok(nitro_texture)
    }
}

// This function will never return 0.  0 can now be used as a null value.
pub fn next_dj_id(app: &mut App) -> u64 {
    app.next_dj_id += 1;
    app.next_dj_id
}

// This function will never return 0.  0 can now be used as a null value.
pub fn next_game_object_id(app: &mut App) -> u64 {
    app.next_game_object_id += 1;
    app.next_game_object_id
}

// Fetches sound from cache if present, otherwise loads it from the filesystem.
pub fn fetch_sound(app: &mut App, path: &str) -> BufferedAudioFile {
    if !app.sound_cache.contains_key(path) {
        let mut sound_path = PathBuf::from("assets");
        sound_path.push("sounds");
        sound_path.push(path);
        let file = File::open(sound_path).unwrap();
        let new_source = rodio::Decoder::new(BufReader::new(file)).unwrap().buffered();
        app.sound_cache.insert(path.to_owned(), Box::new(new_source.clone()));
        return new_source;
    }
    (**app.sound_cache.get(path).unwrap()).clone()
}
