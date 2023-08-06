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
            BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingType, BlendState, BufferSize, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, MultisampleState, Operations,
            PipelineCache, PolygonMode, PrimitiveState, PrimitiveTopology,
            RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, ShaderStage,
            ShaderStages, ShaderType, TextureFormat, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ExtractedView, ViewTarget},
        RenderApp,
    },
};
use std::time::Duration;

fn main() {
    let mut app = App::new();

    // enable hot-loading so our shader gets reloaded when it changes
    app.add_plugins((
        DefaultPlugins.set(AssetPlugin {
            watch_for_changes: ChangeWatcher::with_delay(Duration::from_millis(200)),
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

#[derive(Debug, ShaderType, Default)]
struct ViewUniform {
    viewport: UVec4,
    projection: Mat4,
}

#[derive(Debug, Default)]
struct HexGridRenderNode;

impl HexGridRenderNode {
    pub const NAME: &str = "hex_grid";
}

impl ViewNode for HexGridRenderNode {
    type ViewQuery = (&'static ExtractedView, &'static ViewTarget);

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        (view, view_target): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let hex_grid_pipeline = world.resource::<HexGridPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache
            .get_render_pipeline(hex_grid_pipeline.pipeline_id)
            .expect("HexGridPipeline should be present in the PipelineCache");

        // create a buffer for our uniform and write it to the GPU
        let mut buffer: UniformBuffer<ViewUniform> = UniformBuffer::default();
        buffer.set(ViewUniform {
            viewport: view.viewport,
            projection: view.projection,
        });
        let render_queue = world.resource::<RenderQueue>();
        buffer.write_buffer(render_context.render_device(), render_queue);
        let view_binding = buffer
            .binding()
            .expect("ViewUniform buffer binding to be valid");

        // create a bind group
        let bind_group = render_context
            .render_device()
            .create_bind_group(&BindGroupDescriptor {
                label: Some("hex_grid_bind_group"),
                layout: &hex_grid_pipeline.layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_binding.clone(),
                }],
            });

        // create a render pass.  Note that we don't want to inherit the
        // color_attachments because then the pipeline Multisample must match
        // whatever msaa was set to.
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("hex_grid_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: view_target.main_texture_view(),
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..4, 0..1);
        Ok(())
    }
}

#[derive(Debug, Resource)]
struct HexGridPipeline {
    pipeline_id: CachedRenderPipelineId,
    layout: BindGroupLayout,
}

impl FromWorld for HexGridPipeline {
    fn from_world(world: &mut World) -> Self {
        let shader = world.resource::<AssetServer>().load("hex_grid.wgsl");

        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("hex_grid_bind_group_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: bevy::render::render_resource::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_cache = world.resource_mut::<PipelineCache>();

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("hex_grid_pipeline".into()),
            layout: vec![layout.clone()],
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
            multisample: MultisampleState::default(),
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

        Self {
            pipeline_id,
            layout,
        }
    }
}
