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
    core_pipeline::{core_3d, tonemapping::Tonemapping},
    ecs::query::QueryItem,
    prelude::*,
    reflect::TypePath,
    render::{
        camera::ScalingMode,
        render_graph::{RenderGraphApp, ViewNode, ViewNodeRunner},
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingType, BlendState, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState,
            FragmentState, LoadOp, MultisampleState, Operations, PipelineCache, PolygonMode,
            PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
            RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
            ShaderStages, ShaderType, StencilFaceState, StencilState, TextureFormat, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ExtractedView, ViewDepthTexture, ViewTarget},
        RenderApp,
    },
};
use bevy_dolly::prelude::*;
use bevy_inspector_egui::quick::FilterQueryInspectorPlugin;
use leafwing_input_manager::prelude::*;
use std::time::Duration;

fn main() {
    let mut app = App::new();

    // enable hot-loading so our shader gets reloaded when it changes
    app.add_plugins((
        DefaultPlugins.set(AssetPlugin {
            watch_for_changes: ChangeWatcher::with_delay(Duration::from_millis(200)),
            ..default()
        }),
        InputManagerPlugin::<InputActions>::default(),
        FilterQueryInspectorPlugin::<With<MainCamera>>::default(),
        HexGridPlugin,
    ))
    .add_systems(Startup, setup)
    .add_systems(Update, (Dolly::<MainCamera>::update_active, handle_input))
    .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add a cube so it's really clear when the shader doesn't run
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(StandardMaterial::default()),
        ..default()
    });

    // camera
    commands.spawn((
        Name::new("Camera"),
        MainCamera,
        Camera3dBundle {
            tonemapping: Tonemapping::None,
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::WindowSize(96.0),
                ..default()
            }
            .into(),
            ..default()
        },
        InputManagerBundle::<InputActions> {
            action_state: ActionState::default(),
            input_map: input_map(),
        },
        Rig::builder()
            .with(Position::new(Vec3::new(0.0, 0.0, 0.0)))
            .with(YawPitch::new().pitch_degrees(-30.0).yaw_degrees(45.0))
            .with(Smooth::new_position(0.3))
            .with(Smooth::new_rotation(0.3))
            .with(Arm::new(Vec3::Z * 100.0))
            .build(),
    ));
}

#[derive(Component)]
struct MainCamera;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, TypePath)]
pub enum InputActions {
    Click,
    Rotate,
    Scale,
    ResetCamera,
    ZeroCamera,
}

#[rustfmt::skip]
fn input_map() -> InputMap<InputActions> {
    InputMap::default()
        .insert(MouseButton::Left, InputActions::Click)
        .insert(DualAxis::mouse_motion(), InputActions::Rotate)
        .insert(SingleAxis::mouse_wheel_y(), InputActions::Scale)
        .insert(KeyCode::Z, InputActions::ResetCamera)
        .insert(KeyCode::Key0, InputActions::ZeroCamera)
        .build()
}

fn handle_input(
    mut camera: Query<(&mut Rig, &mut Projection, &ActionState<InputActions>), With<MainCamera>>,
) {
    let (mut rig, mut _projection, actions) = camera.single_mut();
    let camera_yp = rig.driver_mut::<YawPitch>();
    // let Projection::Orthographic(projection) = projection.as_mut() else { panic!("wrong scaling mode") };

    if actions.just_pressed(InputActions::ResetCamera) {
        camera_yp.yaw_degrees = 45.0;
        camera_yp.pitch_degrees = -30.0;
        //projection.scale = 1.0;
    }

    if actions.just_pressed(InputActions::ZeroCamera) {
        camera_yp.yaw_degrees = 0.0;
        camera_yp.pitch_degrees = 0.0;
        // projection.scale = 1.0;
    }

    if actions.pressed(InputActions::Click) {
        let vector = actions.axis_pair(InputActions::Rotate).unwrap().xy();
        camera_yp.rotate_yaw_pitch(-0.1 * vector.x * 15.0, vector.y);
    }

    let scale = actions.value(InputActions::Scale);
    if scale != 0.0 {
        // projection.scale = (projection.scale * (1.0 - scale * 0.005)).clamp(0.001, 15.0);
    }
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
    inverse_projection: Mat4,
    view: Mat4,
    inverse_view: Mat4,
    position: Vec3,
}

#[derive(Debug, Default)]
struct HexGridRenderNode;

impl HexGridRenderNode {
    pub const NAME: &str = "hex_grid";
}

impl ViewNode for HexGridRenderNode {
    type ViewQuery = (
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    );

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        (view, view_target, depth): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let hex_grid_pipeline = world.resource::<HexGridPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache
            .get_render_pipeline(hex_grid_pipeline.pipeline_id)
            .expect("HexGridPipeline should be present in the PipelineCache");

        // create a buffer for our uniform and write it to the GPU
        let mut buffer: UniformBuffer<ViewUniform> = UniformBuffer::default();
        let view_matrix = view.transform.compute_matrix();
        buffer.set(ViewUniform {
            viewport: view.viewport,
            projection: view.projection,
            inverse_projection: view.projection.inverse(),
            view: view_matrix,
            inverse_view: view_matrix.inverse(),
            position: view.transform.translation(),
        });
        // let mat = dbg!(view.projection * view.transform.compute_matrix());
        // let vecs = [
        //     Vec4::new(-1., -1., 0., 1.),
        //     Vec4::new(-1., 1., 0., 1.),
        //     Vec4::new(1., -1., 0., 1.),
        //     Vec4::new(1., 1., 0., 1.),
        // ];
        // for vec in vecs {
        //     debug!("{:?} -> {:?}", vec, mat * vec);
        // }
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
            color_attachments: &[Some(view_target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            }))],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth.view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            }),
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
                visibility: ShaderStages::VERTEX_FRAGMENT,
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
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
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

        Self {
            pipeline_id,
            layout,
        }
    }
}
