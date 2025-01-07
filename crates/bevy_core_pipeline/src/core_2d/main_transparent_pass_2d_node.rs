use crate::core_2d::Transparent2d;
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::ViewSortedRenderPhases,
    render_resource::RenderPassDescriptor,
    renderer::RenderContext,
    view::ViewTarget,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

#[derive(Default)]
pub struct MainTransparentPass2dNode {}

impl ViewNode for MainTransparentPass2dNode {
    type ViewQuery = (&'static ExtractedCamera, &'static ViewTarget);

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, target): bevy_ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(transparent_phases) =
            world.get_resource::<ViewSortedRenderPhases<Transparent2d>>()
        else {
            return Ok(());
        };

        let view_entity = graph.view_entity();
        let Some(transparent_phase) = transparent_phases.get(&view_entity) else {
            return Ok(());
        };

        // This needs to run at least once to clear the background color, even if there are no items to render
        {
            #[cfg(feature = "trace")]
            let _main_pass_2d = info_span!("main_transparent_pass_2d").entered();

            let diagnostics = render_context.diagnostic_recorder();

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("main_transparent_pass_2d"),
                color_attachments: &[Some(target.get_color_attachment())],
                // TODO 这里其实还有问题，需要提供一个开关或者判断条件，因为并不是所有的 transparent 2d 的 pipeline 都使用 stencil 的，如果 pipeline 没有定义 stencil 这里会报错的
                depth_stencil_attachment: Some(target.get_stencil_attachment()),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let pass_span = diagnostics.pass_span(&mut render_pass, "main_transparent_pass_2d");

            pass_span.end(&mut render_pass);
        }

        transparent_phase.render_with_grab(render_context, world, view_entity, camera, target);

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        if camera.viewport.is_some() {
            #[cfg(feature = "trace")]
            let _reset_viewport_pass_2d = info_span!("reset_viewport_pass_2d").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("reset_viewport_pass_2d"),
                color_attachments: &[Some(target.get_color_attachment())],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            render_context
                .command_encoder()
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}
