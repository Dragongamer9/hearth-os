use smithay::{
    backend::renderer::{
        damage::{Error as OutputDamageTrackerError, OutputDamageTracker, RenderOutputResult},
        element::{
            memory::MemoryRenderBufferRenderElement,
            surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
            utils::{
                ConstrainAlign, ConstrainScaleBehavior, CropRenderElement, RelocateRenderElement,
                RescaleRenderElement,
            },
            AsRenderElements, Kind, RenderElement, Wrap,
        },
        gles::{GlesFrame, GlesRenderer, GlesTexProgram, Uniform, UniformName, UniformType},
        utils::{CommitCounter, DamageSet, OpaqueRegions},
        Color32F, ImportAll, ImportMem, Renderer, RendererSuper,
    },
    desktop::space::{
        constrain_space_element, ConstrainBehavior, ConstrainReference, Space, SpaceRenderElements,
    },
    output::Output,
    utils::{Buffer as BufferCoords, Physical, Point, Rectangle, Scale, Size, Transform},
};

#[cfg(feature = "debug")]
use crate::drawing::FpsElement;
use crate::{
    drawing::{clear_color, PointerRenderElement, CLEAR_COLOR_FULLSCREEN},
    lock::LockView,
    shell::{FullscreenSurface, WindowElement, WindowRenderElement},
};

smithay::backend::renderer::element::render_elements! {
    pub CustomRenderElements<R> where
        R: ImportAll + ImportMem;
    Pointer=PointerRenderElement<R>,
    Surface=WaylandSurfaceRenderElement<R>,
    Memory=MemoryRenderBufferRenderElement<R>,
    #[cfg(feature = "debug")]
    // Note: We would like to borrow this element instead, but that would introduce
    // a feature-dependent lifetime, which introduces a lot more feature bounds
    // as the whole type changes and we can't have an unused lifetime (for when "debug" is disabled)
    // in the declaration.
    Fps=FpsElement<R::TextureId>,
}

impl<R: Renderer> std::fmt::Debug for CustomRenderElements<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pointer(arg0) => f.debug_tuple("Pointer").field(arg0).finish(),
            Self::Surface(arg0) => f.debug_tuple("Surface").field(arg0).finish(),
            Self::Memory(arg0) => f.debug_tuple("Memory").field(arg0).finish(),
            #[cfg(feature = "debug")]
            Self::Fps(arg0) => f.debug_tuple("Fps").field(arg0).finish(),
            Self::_GenericCatcher(arg0) => f.debug_tuple("_GenericCatcher").field(arg0).finish(),
        }
    }
}

smithay::backend::renderer::element::render_elements! {
    pub OutputRenderElements<R, E> where R: ImportAll + ImportMem + GameFilter;
    Space=SpaceRenderElements<R, E>,
    Window=Wrap<E>,
    Custom=CustomRenderElements<R>,
    Preview=CropRenderElement<RelocateRenderElement<RescaleRenderElement<WindowRenderElement<R>>>>,
    Game=SharpenElement<R>,
}

impl<R: Renderer + ImportAll + ImportMem + GameFilter, E: RenderElement<R> + std::fmt::Debug> std::fmt::Debug
    for OutputRenderElements<R, E>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Space(arg0) => f.debug_tuple("Space").field(arg0).finish(),
            Self::Window(arg0) => f.debug_tuple("Window").field(arg0).finish(),
            Self::Custom(arg0) => f.debug_tuple("Custom").field(arg0).finish(),
            Self::Preview(arg0) => f.debug_tuple("Preview").field(arg0).finish(),
            Self::Game(arg0) => f.debug_tuple("Game").field(arg0).finish(),
            Self::_GenericCatcher(arg0) => f.debug_tuple("_GenericCatcher").field(arg0).finish(),
        }
    }
}

pub fn space_preview_elements<'a, R, C>(
    renderer: &'a mut R,
    space: &'a Space<WindowElement>,
    output: &'a Output,
) -> impl Iterator<Item = C> + 'a
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + Send + 'static,
    C: From<CropRenderElement<RelocateRenderElement<RescaleRenderElement<WindowRenderElement<R>>>>> + 'a,
{
    let constrain_behavior = ConstrainBehavior {
        reference: ConstrainReference::BoundingBox,
        behavior: ConstrainScaleBehavior::Fit,
        align: ConstrainAlign::CENTER,
    };

    let preview_padding = 10;

    let elements_on_space = space.elements_for_output(output).count();
    let output_scale = output.current_scale().fractional_scale();
    let output_transform = output.current_transform();
    let output_size = output
        .current_mode()
        .map(|mode| {
            output_transform
                .transform_size(mode.size)
                .to_f64()
                .to_logical(output_scale)
        })
        .unwrap_or_default();

    let max_elements_per_row = 4;
    let elements_per_row = usize::min(elements_on_space, max_elements_per_row);
    let rows = f64::ceil(elements_on_space as f64 / elements_per_row as f64);

    let preview_size = Size::from((
        f64::round(output_size.w / elements_per_row as f64) as i32 - preview_padding * 2,
        f64::round(output_size.h / rows) as i32 - preview_padding * 2,
    ));

    space
        .elements_for_output(output)
        .enumerate()
        .flat_map(move |(element_index, window)| {
            let column = element_index % elements_per_row;
            let row = element_index / elements_per_row;
            let preview_location = Point::from((
                preview_padding + (preview_padding + preview_size.w) * column as i32,
                preview_padding + (preview_padding + preview_size.h) * row as i32,
            ));
            let constrain = Rectangle::new(preview_location, preview_size);
            constrain_space_element(
                renderer,
                window,
                preview_location,
                1.0,
                output_scale,
                constrain,
                constrain_behavior,
            )
        })
}

#[profiling::function]
pub fn output_elements<R>(
    output: &Output,
    space: &Space<WindowElement>,
    custom_elements: impl IntoIterator<Item = CustomRenderElements<R>>,
    renderer: &mut R,
    show_window_preview: bool,
    lock: Option<&LockView>,
) -> (Vec<OutputRenderElements<R, WindowRenderElement<R>>>, Color32F)
where
    R: Renderer + ImportAll + ImportMem + GameFilter,
    R::TextureId: Clone + Send + 'static,
{
    // Locked (lock.rs): the lock screen (or the built-in prompt) and the pointer, nothing else, ever.
    if let Some(view) = lock {
        let scale = output.current_scale().fractional_scale();
        let mut elements: Vec<OutputRenderElements<R, WindowRenderElement<R>>> =
            custom_elements.into_iter().map(OutputRenderElements::from).collect();
        match view {
            LockView::Surface(surface) => {
                let surf: Vec<WaylandSurfaceRenderElement<R>> = render_elements_from_surface_tree(
                    renderer,
                    surface,
                    (0, 0),
                    scale,
                    1.0,
                    Kind::Unspecified,
                );
                elements.extend(surf.into_iter().map(|e| OutputRenderElements::Custom(CustomRenderElements::Surface(e))));
            }
            LockView::Prompt(buffer) => {
                if let Ok(e) = MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    (0.0, 0.0),
                    buffer,
                    None,
                    None,
                    None,
                    Kind::Unspecified,
                ) {
                    elements.push(OutputRenderElements::Custom(CustomRenderElements::Memory(e)));
                }
            }
            LockView::Cover => {}
        }
        return (elements, clear_color());
    }
    if let Some(window) = output
        .user_data()
        .get::<FullscreenSurface>()
        .and_then(|f| f.get())
    {
        let scale = output.current_scale().fractional_scale().into();
        let window_render_elements: Vec<WindowRenderElement<R>> =
            AsRenderElements::<R>::render_elements(&window, renderer, (0, 0).into(), scale, 1.0);
        // An upscaled game (game.rs): sharpened as it's drawn, when the shader could be made.
        let sharpen = crate::game::sharpen_for(output).filter(|_| renderer.prepare_sharpen());

        let elements = custom_elements
            .into_iter()
            .map(OutputRenderElements::from)
            .chain(window_render_elements.into_iter().map(|e| match sharpen {
                Some(strength) => OutputRenderElements::Game(SharpenElement { inner: e, strength }),
                None => OutputRenderElements::Window(Wrap::from(e)),
            }))
            .collect::<Vec<_>>();
        (elements, CLEAR_COLOR_FULLSCREEN)
    } else {
        let mut output_render_elements = custom_elements
            .into_iter()
            .map(OutputRenderElements::from)
            .collect::<Vec<_>>();

        if show_window_preview && space.elements_for_output(output).count() > 0 {
            output_render_elements.extend(space_preview_elements(renderer, space, output));
        }

        let space_elements = smithay::desktop::space::space_render_elements::<_, WindowElement, _>(
            renderer,
            [space],
            output,
            1.0,
        )
        .expect("output without mode?");
        output_render_elements.extend(space_elements.into_iter().map(OutputRenderElements::Space));

        (output_render_elements, clear_color())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_output<'a, 'd, R>(
    output: &'a Output,
    space: &'a Space<WindowElement>,
    custom_elements: impl IntoIterator<Item = CustomRenderElements<R>>,
    renderer: &'a mut R,
    framebuffer: &'a mut R::Framebuffer<'_>,
    damage_tracker: &'d mut OutputDamageTracker,
    age: usize,
    show_window_preview: bool,
    lock: Option<&LockView>,
) -> Result<RenderOutputResult<'d>, OutputDamageTrackerError<R::Error>>
where
    R: Renderer + ImportAll + ImportMem + GameFilter,
    R::TextureId: Clone + Send + 'static,
{
    let (elements, clear_color) =
        output_elements(output, space, custom_elements, renderer, show_window_preview, lock);
    damage_tracker.render_output(renderer, framebuffer, age, &elements, clear_color)
}

// ---------------------------------------------------------------------------------------------- an upscaled game

/// Renderers that can sharpen an upscaled game (GLES, and the multi-GPU renderer over GLES on real screens). Others
/// draw it with the plain linear filter.
pub trait GameFilter: Renderer {
    /// Makes the sharpening shader for this renderer's GL context, once. False when it can't be had.
    fn prepare_sharpen(&mut self) -> bool {
        false
    }
    /// Sharpening on for the textures drawn next; false when it isn't available.
    fn sharpen_on(_frame: &mut <Self as RendererSuper>::Frame<'_, '_>, _texel: [f32; 2], _strength: f32) -> bool {
        false
    }
    fn sharpen_off(_frame: &mut <Self as RendererSuper>::Frame<'_, '_>) {}
}

impl GameFilter for GlesRenderer {
    fn prepare_sharpen(&mut self) -> bool {
        gles_prepare_sharpen(self)
    }
    fn sharpen_on(frame: &mut <Self as RendererSuper>::Frame<'_, '_>, texel: [f32; 2], strength: f32) -> bool {
        gles_sharpen_on(frame, texel, strength)
    }
    fn sharpen_off(frame: &mut <Self as RendererSuper>::Frame<'_, '_>) {
        frame.clear_tex_program_override();
    }
}

/// The sharpening program, kept with its GL context (None: it couldn't be compiled; plain linear is used).
struct SharpenProgram(Option<GlesTexProgram>);

/// One pass over the linearly upscaled texture: a contrast-adaptive sharpen in the style of FSR 1's RCAS / AMD's CAS.
/// Each pixel looks at its four neighbours one source pixel away and sharpens less where the contrast is already
/// high, so edges don't ring. Same inputs and defines as Smithay's own texture shader.
const SHARPEN_SHADER: &str = r#"#version 100

//_DEFINES_

#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif

precision mediump float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif

uniform float alpha;
uniform vec2 texel;
uniform float sharpness;
varying vec2 v_coords;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

void main() {
    vec4 c = texture2D(tex, v_coords);
    vec3 n = texture2D(tex, v_coords - vec2(0.0, texel.y)).rgb;
    vec3 s = texture2D(tex, v_coords + vec2(0.0, texel.y)).rgb;
    vec3 w = texture2D(tex, v_coords - vec2(texel.x, 0.0)).rgb;
    vec3 e = texture2D(tex, v_coords + vec2(texel.x, 0.0)).rgb;
    vec3 mn = min(c.rgb, min(min(n, s), min(w, e)));
    vec3 mx = max(c.rgb, max(max(n, s), max(w, e)));
    vec3 amp = sqrt(clamp(min(mn, vec3(2.0) - mx) / max(mx, vec3(0.0001)), 0.0, 1.0));
    vec3 wt = amp * mix(-0.125, -0.2, sharpness);
    vec3 rgb = (c.rgb + (n + s + w + e) * wt) / (vec3(1.0) + 4.0 * wt);
    vec4 color = vec4(clamp(rgb, 0.0, 1.0), c.a);

#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0) * alpha;
#else
    color = color * alpha;
#endif

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
"#;

pub fn gles_prepare_sharpen(renderer: &mut GlesRenderer) -> bool {
    if let Some(p) = renderer.egl_context().user_data().get::<SharpenProgram>() {
        return p.0.is_some();
    }
    let program = match renderer.compile_custom_texture_shader(
        SHARPEN_SHADER,
        &[UniformName::new("texel", UniformType::_2f), UniformName::new("sharpness", UniformType::_1f)],
    ) {
        Ok(p) => {
            tracing::info!("Firehold game: the sharpening shader is ready");
            Some(p)
        }
        Err(e) => {
            tracing::warn!(err = ?e, "Firehold game: no sharpening shader; upscaled games are drawn with the plain linear filter");
            None
        }
    };
    let ok = program.is_some();
    renderer.egl_context().user_data().insert_if_missing(|| SharpenProgram(program));
    ok
}

pub fn gles_sharpen_on(frame: &mut GlesFrame<'_, '_>, texel: [f32; 2], strength: f32) -> bool {
    let Some(program) = frame.egl_context().user_data().get::<SharpenProgram>().and_then(|p| p.0.clone()) else {
        return false;
    };
    frame.override_default_tex_program(
        program,
        vec![Uniform::new("texel", texel), Uniform::new("sharpness", strength)],
    );
    true
}

/// The fullscreen game's picture, drawn through the sharpening shader. Never scanned out directly (it needs the GPU).
pub struct SharpenElement<R: Renderer + ImportAll + ImportMem> {
    pub inner: WindowRenderElement<R>,
    pub strength: f32,
}

impl<R: Renderer + ImportAll + ImportMem> std::fmt::Debug for SharpenElement<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharpenElement").field("inner", &self.inner).field("strength", &self.strength).finish()
    }
}

impl<R> smithay::backend::renderer::element::Element for SharpenElement<R>
where
    R: Renderer + ImportAll + ImportMem,
    <R as RendererSuper>::TextureId: 'static,
{
    fn id(&self) -> &smithay::backend::renderer::element::Id {
        self.inner.id()
    }
    fn current_commit(&self) -> CommitCounter {
        self.inner.current_commit()
    }
    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.inner.location(scale)
    }
    fn src(&self) -> Rectangle<f64, BufferCoords> {
        self.inner.src()
    }
    fn transform(&self) -> Transform {
        self.inner.transform()
    }
    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }
    fn damage_since(&self, scale: Scale<f64>, commit: Option<CommitCounter>) -> DamageSet<i32, Physical> {
        self.inner.damage_since(scale, commit)
    }
    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.inner.opaque_regions(scale)
    }
    fn alpha(&self) -> f32 {
        self.inner.alpha()
    }
    fn kind(&self) -> Kind {
        self.inner.kind()
    }
}

impl<R> RenderElement<R> for SharpenElement<R>
where
    R: Renderer + ImportAll + ImportMem + GameFilter,
    <R as RendererSuper>::TextureId: 'static,
{
    fn draw(
        &self,
        frame: &mut <R as RendererSuper>::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), <R as RendererSuper>::Error> {
        // One source pixel, in texture coordinates (src is the whole buffer for a game's surface).
        let texel = [1.0 / src.size.w.max(1.0) as f32, 1.0 / src.size.h.max(1.0) as f32];
        let on = R::sharpen_on(frame, texel, self.strength);
        let res = self.inner.draw(frame, src, dst, damage, opaque_regions);
        if on {
            R::sharpen_off(frame);
        }
        res
    }
}
