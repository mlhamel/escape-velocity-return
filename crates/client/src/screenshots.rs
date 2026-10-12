use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

#[derive(Component)]
struct Capturing;

pub struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (manual_capture_keybind, cleanup_captures).chain());
    }
}

pub fn request_capture(commands: &mut Commands, path: String) {
    commands
        .spawn((Screenshot::primary_window(), Capturing))
        .observe(
            move |captured: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                save_to_disk(path.clone())(captured);
            },
        );
}

fn cleanup_captures(
    mut commands: Commands,
    done: Query<Entity, (With<Capturing>, Without<Screenshot>)>,
) {
    for entity in &done {
        commands.entity(entity).despawn();
    }
}

fn manual_capture_keybind(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keys.just_pressed(KeyCode::F12) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        request_capture(&mut commands, format!("screenshot-{timestamp}.png"));
    }
}
