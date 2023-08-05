/// minimal example of adding a custom render pipeline in bevy 0.11.
///
/// When this example runs, you should only see a blue screen. There are no
/// vertex buffers, or anything else in this example.  Effectively it is
/// shader-toy written in bevy.
///
/// If no messages appear on stdout, set to help debug:
///     RUST_LOG="info,wgpu_core=warn,wgpu_hal=warn"
///
/// See comments throughout file for more details.
///
use bevy::{
    asset::ChangeWatcher,
    core_pipeline::core_3d,
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{RenderGraphApp, ViewNode, ViewNodeRunner},
        render_resource::{
            BlendState, CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState,
            LoadOp, MultisampleState, Operations, PipelineCache, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPassDescriptor, RenderPipelineDescriptor, TextureFormat,
        },
        texture::BevyDefault,
        view::ViewTarget,
        RenderApp,
    },
};
use std::time::Duration;

fn main() {
    let mut app = App::new();

    // enable hot-loading so our shader gets reloaded when it changes
    app.add_plugins((
        DefaultPlugins.set(AssetPlugin {
            watch_for_changes: ChangeWatcher::with_delay(Duration::from_secs(1)),
            ..default()
        }),
        HexGridPlugin,
    ))
    .add_systems(Startup, setup)
    .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add a cube so it's really clear when the shader doesn't run
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.5 })),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(StandardMaterial::default()),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

struct HexGridPlugin;

impl Plugin for HexGridPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app
            .get_sub_app_mut(RenderApp)
            .expect("RenderApp should already exist in App");

        // add our post-processing render node to the render graph
        // place it between tonemapping & the end of post-processing shaders
        render_app
            .add_render_graph_node::<ViewNodeRunner<HexGridRenderNode>>(
                core_3d::graph::NAME,
                HexGridRenderNode::NAME,
            )
            .add_render_graph_edges(
                core_3d::graph::NAME,
                &[
                    core_3d::graph::node::TONEMAPPING,
                    HexGridRenderNode::NAME,
                    core_3d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                ],
            );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app
            .get_sub_app_mut(RenderApp)
            .expect("RenderApp should already exist in App");
        render_app.init_resource::<HexGridPipeline>();
    }
}

#[derive(Debug, Default)]
struct HexGridRenderNode;

impl HexGridRenderNode {
    pub const NAME: &str = "hex_grid";
}

impl ViewNode for HexGridRenderNode {
    type ViewQuery = (&'static ExtractedCamera, &'static ViewTarget);
    fn run(
        &self,
        graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        (camera, target): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let hex_grid_pipeline = world.resource::<HexGridPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache
            .get_render_pipeline(hex_grid_pipeline.pipeline_id)
            .expect("HexGridPipeline should be present in the PipelineCache");

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("hex_grid_pass"),
            color_attachments: &[Some(target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            }))],
            depth_stencil_attachment: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.draw(0..4, 0..1);
        debug!("{:?} ran", self);
        Ok(())
    }
}

#[derive(Debug, Resource)]
struct HexGridPipeline {
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for HexGridPipeline {
    fn from_world(world: &mut World) -> Self {
        let shader = world.resource::<AssetServer>().load("hex_grid.wgsl");

        let pipeline_cache = world.resource_mut::<PipelineCache>();

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("hex_grid_pipeline".into()),
            layout: vec![],
            push_constant_ranges: Vec::new(),
            vertex: bevy::render::render_resource::VertexState {
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![],
            },
            // default does not work here as we're using TriangleStrip
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: bevy::render::render_resource::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            // XXX hard-coding this for right now, but this is wrong.  The post
            // processing example uses default() and works fine.  But when we
            // try to run our pipeline, we get errors about msaa mismatch.
            // One key difference is that we're not calling
            // view_target.post_process_write() in our `run()`.
            //
            // multisample: MultisampleState::default(),
            multisample: MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
        });

        Self { pipeline_id }
    }
}
