use bevy::{
    asset::ChangeWatcher,
    core_pipeline::core_3d::Transparent3d,
    ecs::{query::ROQueryItem, system::SystemParamItem},
    prelude::*,
    render::{
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline,
        },
        render_resource::*,
        texture::BevyDefault,
        view::VisibleEntities,
        Extract, Render, RenderApp, RenderSet,
    },
};
use std::{borrow::Cow, time::Duration};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins.set(AssetPlugin {
        watch_for_changes: ChangeWatcher::with_delay(Duration::from_secs(1)),
        ..default()
    }),))
        .add_systems(Startup, setup);

    let render_app = app.get_sub_app_mut(RenderApp).unwrap();
    render_app
        .init_resource::<ShaderToyPipeline>()
        .init_resource::<SpecializedRenderPipelines<ShaderToyPipeline>>()
        .add_render_command::<Transparent3d, DrawShaderToy>()
        .add_systems(ExtractSchedule, extract_shader_toys)
        .add_systems(Render, queue_shader_toys.in_set(RenderSet::Queue));

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
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

    // add our shader toy
    let shader = asset_server.load("shaders/custom_material.wgsl");
    commands.spawn((SpatialBundle::default(), ShaderToy { shader }));
}

#[derive(Debug, Component, Clone, Hash, PartialEq, Eq)]
struct ShaderToy {
    shader: Handle<Shader>,
}

#[derive(Resource, Default)]
struct ShaderToyPipeline;

impl SpecializedRenderPipeline for ShaderToyPipeline {
    type Key = ShaderToy;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some(Cow::Borrowed("shader-toy-pipeline")),
            layout: vec![],
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: key.shader.clone(),
                shader_defs: vec![],
                entry_point: Cow::Borrowed("vertex"),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: key.shader,
                shader_defs: vec![],
                entry_point: Cow::Borrowed("fragment"),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: bevy::render::render_resource::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            // NOTE: `depth_stencil` & `multisample` fields must have the same
            // settings as tne `RenderPass` (`Transparent3d`), otherwise the
            // shader will fail to compile.
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
        }
    }
}

type DrawShaderToy = (SetItemPipeline, FinishDrawShaderToy);

struct FinishDrawShaderToy;

impl<P: PhaseItem> RenderCommand<P> for FinishDrawShaderToy {
    type Param = ();
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        _entity: ROQueryItem<'w, Self::ItemWorldQuery>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.draw(0..4, 0..1);
        RenderCommandResult::Success
    }
}

fn extract_shader_toys(mut commands: Commands, shader_toys: Extract<Query<(Entity, &ShaderToy)>>) {
    let extracted: Vec<(Entity, ShaderToy)> = shader_toys
        .iter()
        .map(|(entity, shader_toy)| (entity, shader_toy.clone()))
        .collect();
    commands.insert_or_spawn_batch(extracted);
}

fn queue_shader_toys(
    pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<ShaderToyPipeline>>,
    pipeline: Res<ShaderToyPipeline>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut views: Query<(&VisibleEntities, &mut RenderPhase<Transparent3d>)>,
    shader_toys: Query<&ShaderToy>,
) {
    let draw_function = transparent_draw_functions
        .read()
        .get_id::<DrawShaderToy>()
        .unwrap();

    for (entities, mut phase) in &mut views {
        for &entity in &entities.entities {
            let Ok(shader_toy) = shader_toys.get(entity) else { continue; };
            let pipeline = pipelines.specialize(&pipeline_cache, &pipeline, shader_toy.clone());
            phase.items.push(Transparent3d {
                pipeline,
                entity,
                draw_function,
                distance: f32::NEG_INFINITY,
            });
        }
    }
}
