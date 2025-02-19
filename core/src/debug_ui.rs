mod display_object;
mod handle;

use crate::context::{RenderContext, UpdateContext};
use crate::debug_ui::display_object::DisplayObjectWindow;
use crate::debug_ui::handle::DisplayObjectHandle;
use crate::display_object::TDisplayObject;
use gc_arena::DynamicRootSet;
use hashbrown::HashMap;
use ruffle_render::commands::CommandHandler;
use ruffle_render::matrix::Matrix;
use swf::{Color, Rectangle, Twips};

#[derive(Default)]
pub struct DebugUi {
    display_objects: HashMap<DisplayObjectHandle, DisplayObjectWindow>,
    queued_messages: Vec<Message>,
}

#[derive(Debug)]
pub enum Message {
    TrackDisplayObject(DisplayObjectHandle),
    TrackStage,
}

impl DebugUi {
    pub fn show(&mut self, egui_ctx: &egui::Context, context: &mut UpdateContext) {
        let mut messages = std::mem::take(&mut self.queued_messages);
        self.display_objects.retain(|object, window| {
            let object = object.fetch(context.dynamic_root);
            window.show(egui_ctx, context, object, &mut messages)
        });
        for message in messages {
            match message {
                Message::TrackDisplayObject(object) => self.track_display_object(object),
                Message::TrackStage => {
                    self.track_display_object(DisplayObjectHandle::new(context, context.stage))
                }
            }
        }
    }

    pub fn queue_message(&mut self, message: Message) {
        self.queued_messages.push(message);
    }

    pub fn track_display_object(&mut self, handle: DisplayObjectHandle) {
        self.display_objects.insert(handle, Default::default());
    }

    pub fn draw_debug_rects<'gc>(
        &self,
        context: &mut RenderContext<'_, 'gc>,
        dynamic_root_set: DynamicRootSet<'gc>,
    ) {
        let world_matrix = context.stage.view_matrix() * *context.stage.base().matrix();

        for (object, window) in self.display_objects.iter() {
            if let Some(color) = window.debug_rect_color() {
                let object = object.fetch(dynamic_root_set);
                let bounds = world_matrix * object.world_bounds();

                draw_debug_rect(context, color, bounds, 3.0);
            }

            if let Some(object) = window.hovered_debug_rect() {
                let object = object.fetch(dynamic_root_set);
                let bounds = world_matrix * object.world_bounds();

                draw_debug_rect(context, swf::Color::RED, bounds, 5.0);
            }
        }
    }
}

fn draw_debug_rect(
    context: &mut RenderContext,
    color: Color,
    bounds: Rectangle<Twips>,
    thickness: f32,
) {
    let width = bounds.width().to_pixels() as f32;
    let height = bounds.height().to_pixels() as f32;
    let thickness_twips = Twips::from_pixels(thickness as f64);

    // Top
    context.commands.draw_rect(
        color.clone(),
        Matrix::create_box(
            width,
            thickness,
            0.0,
            bounds.x_min,
            bounds.y_min - thickness_twips,
        ),
    );
    // Bottom
    context.commands.draw_rect(
        color.clone(),
        Matrix::create_box(width, thickness, 0.0, bounds.x_min, bounds.y_max),
    );
    // Left
    context.commands.draw_rect(
        color.clone(),
        Matrix::create_box(
            thickness,
            height,
            0.0,
            bounds.x_min - thickness_twips,
            bounds.y_min,
        ),
    );
    // Right
    context.commands.draw_rect(
        color,
        Matrix::create_box(thickness, height, 0.0, bounds.x_max, bounds.y_min),
    );
}
