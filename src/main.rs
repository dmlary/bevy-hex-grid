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

    // enable hot-loading so our shader gets reloaded when it changes
    app.add_plugins((DefaultPlugins.set(AssetPlugin {
        watch_for_changes: ChangeWatcher::with_delay(Duration::from_secs(1)),
        ..default()
    }),))
        .add_systems(Startup, setup);

    // Add rendering of ShaderToys to the RenderApp
    let render_app = app.get_sub_app_mut(RenderApp).unwrap();
    render_app
        // the ShaderToyPipeline is used to generate RenderPipelineDescriptor
        // for each shader; see `RenderPipelineDescriptor::specialize()` calls
        // in `queue_shader_toys()`.
        .init_resource::<ShaderToyPipeline>()
        // This is just a cache for pipelines created by the above.
        //
        // NOTE: If your shader is not configurable (requires no
        // specialization), you may be able to use
        // `PipelineCache::queue_render_pipeline()` with a
        // RenderPipelineDescriptor and skip the specialization code.
        .init_resource::<SpecializedRenderPipelines<ShaderToyPipeline>>()
        // This registers DrawShaderToy as a render command in the Transparent3d
        // render pass; it's used in `queue_shader_toys`
        .add_render_command::<Transparent3d, DrawShaderToy>()
        // This copies ShaderToy instances from the World into the render World
        .add_systems(ExtractSchedule, extract_shader_toys)
        // This adds DrawShaderToy commands to the Transparent3d RenderPhase for
        // each ShaderToy in the render World.
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

    // add our shader toy; note that the SpatialBundle is required so that the
    // ShaderToy is visible to the camera.  This can probably be changed by
    // tweaking `queue_shader_toys()`
    let shader = asset_server.load("shaders/custom_material.wgsl");
    commands.spawn((SpatialBundle::default(), ShaderToy { shader }));
}

#[derive(Component, Clone)]
struct ShaderToy {
    shader: Handle<Shader>,
}

#[derive(Resource, Default)]
struct ShaderToyPipeline;

impl SpecializedRenderPipeline for ShaderToyPipeline {
    type Key = Handle<Shader>;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // For more details about this struct, look at the learning wgpu
        // tutorials.
        RenderPipelineDescriptor {
            // probably should change this name to be based on the Key
            label: Some(Cow::Borrowed("shader-toy-pipeline")),
            layout: vec![],
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: key.clone(),
                shader_defs: vec![],
                entry_point: Cow::Borrowed("vertex"),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: key,
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

// This is used to construct the series of RenderCommands used to draw the
// ShaderToy.  We registered this in main() for Transparent3d phase, and we
// use it in `queue_shader_toys()` as the `Transparent3d.draw_command`.
//
// - SetItemPipeline is a bevy RenderCommand for setting the render pipeline.
// - FinishDrawShaderToy is our RenderCommand for issuing the final draw() call.
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
        // Simple draw command, giving vertices 0..4, and only one instance 0.
        // These values are passed to the shader.  See wgpu tutorial, or opengl
        // tutorial for more details on what this means
        pass.draw(0..4, 0..1);
        RenderCommandResult::Success
    }
}

/// copy ShaderToy instances from World into the render World
///
/// Rendering is done against the render world instead of the live world.  See
/// bevy 0.6 upgrade notes for description of why (spoilers: performance).
fn extract_shader_toys(mut commands: Commands, shader_toys: Extract<Query<(Entity, &ShaderToy)>>) {
    let extracted: Vec<(Entity, ShaderToy)> = shader_toys
        .iter()
        .map(|(entity, shader_toy)| (entity, shader_toy.clone()))
        .collect();
    commands.insert_or_spawn_batch(extracted);
}

/// Take the ShaderToy instances in the render world and add draw items to the
/// Transparent3d phase for any that are visible.
fn queue_shader_toys(
    pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<ShaderToyPipeline>>,
    pipeline: Res<ShaderToyPipeline>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut views: Query<(&VisibleEntities, &mut RenderPhase<Transparent3d>)>,
    shader_toys: Query<&ShaderToy>,
) {
    // look up our draw function
    let draw_function = transparent_draw_functions
        .read()
        .get_id::<DrawShaderToy>()
        .unwrap();

    // go through each view (these can be equivalent to cameras)
    for (entities, mut phase) in &mut views {
        // go through all of the visibile entities in the view
        for &entity in &entities.entities {
            // if we have a shader entity visibile to the view...
            let Ok(shader_toy) = shader_toys.get(entity) else { continue; };

            // specialize the pipeline for the specific shader in the ShaderToy
            let pipeline =
                pipelines.specialize(&pipeline_cache, &pipeline, shader_toy.shader.clone());
            // Add the pipeline & draw function `DrawShaderToy` to the
            // Transparent3d phase for rendering.
            //
            // When Transparent3d runs, it will execute the `RenderCommand`s in
            // `DrawShaderToy` using the pipeline provided (the shader in
            // `ShaderToy`).
            phase.items.push(Transparent3d {
                pipeline,
                draw_function,
                entity,
                distance: f32::NEG_INFINITY,
            });
        }
    }
}
