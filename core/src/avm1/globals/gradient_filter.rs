//! flash.filters.GradientBevelFilter and flash.filters.GradientGlowFilter objects

use crate::avm1::clamp::Clamp;
use crate::avm1::function::{Executable, FunctionObject};
use crate::avm1::globals::bevel_filter::BevelFilterType;
use crate::avm1::object::NativeObject;
use crate::avm1::property_decl::{define_properties_on, Declaration};
use crate::avm1::{Activation, ArrayObject, Error, Object, ScriptObject, TObject, Value};
use crate::context::{GcContext, UpdateContext};
use crate::string::{AvmString, WStr};
use gc_arena::{Collect, GcCell, MutationContext};
use swf::Color;

const MAX_COLORS: usize = 16;

#[derive(Clone, Debug, Collect)]
#[collect(require_static)]
struct GradientFilterData {
    distance: f64,
    // TODO: Introduce `Angle<Radians>` struct.
    angle: f64,
    colors: [Color; MAX_COLORS],
    num_colors: usize,
    ratios: Vec<u8>,
    blur_x: f64,
    blur_y: f64,
    // TODO: Introduce unsigned `Fixed8`?
    strength: u16,
    quality: i32,
    type_: BevelFilterType,
    knockout: bool,
}

impl Default for GradientFilterData {
    #[allow(clippy::approx_constant)]
    fn default() -> Self {
        Self {
            distance: 4.0,
            angle: 0.785398163, // ~45 degrees
            colors: Default::default(),
            num_colors: 0,
            ratios: vec![],
            blur_x: 4.0,
            blur_y: 4.0,
            strength: 1 << 8,
            quality: 1,
            type_: BevelFilterType::Inner,
            knockout: false,
        }
    }
}

#[derive(Clone, Debug, Collect)]
#[collect(no_drop)]
#[repr(transparent)]
pub struct GradientFilter<'gc>(GcCell<'gc, GradientFilterData>);

impl<'gc> GradientFilter<'gc> {
    fn new(activation: &mut Activation<'_, 'gc>, args: &[Value<'gc>]) -> Result<Self, Error<'gc>> {
        let gradient_bevel_filter = Self(GcCell::allocate(
            activation.context.gc_context,
            Default::default(),
        ));
        gradient_bevel_filter.set_distance(activation, args.get(0))?;
        gradient_bevel_filter.set_angle(activation, args.get(1))?;
        gradient_bevel_filter.set_colors(activation, args.get(2))?;
        gradient_bevel_filter.set_alphas(activation, args.get(3))?;
        gradient_bevel_filter.set_ratios(activation, args.get(4))?;
        gradient_bevel_filter.set_blur_x(activation, args.get(5))?;
        gradient_bevel_filter.set_blur_y(activation, args.get(6))?;
        gradient_bevel_filter.set_strength(activation, args.get(7))?;
        gradient_bevel_filter.set_quality(activation, args.get(8))?;
        gradient_bevel_filter.set_type(activation, args.get(9))?;
        gradient_bevel_filter.set_knockout(activation, args.get(10))?;
        Ok(gradient_bevel_filter)
    }

    pub(crate) fn duplicate(&self, gc_context: MutationContext<'gc, '_>) -> Self {
        Self(GcCell::allocate(gc_context, self.0.read().clone()))
    }

    fn distance(&self) -> f64 {
        self.0.read().distance
    }

    fn set_distance(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(value) = value {
            let distance = value.coerce_to_f64(activation)?;
            self.0.write(activation.context.gc_context).distance = distance;
        }
        Ok(())
    }

    fn angle(&self) -> f64 {
        self.0.read().angle.to_degrees()
    }

    fn set_angle(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(value) = value {
            let angle = (value.coerce_to_f64(activation)? % 360.0).to_radians();
            self.0.write(activation.context.gc_context).angle = angle;
        }
        Ok(())
    }

    fn colors(&self, context: &mut UpdateContext<'_, 'gc>) -> ArrayObject<'gc> {
        let read = self.0.read();
        ArrayObject::new(
            context.gc_context,
            context.avm1.prototypes().array,
            read.colors[..read.num_colors]
                .iter()
                .map(|c| c.to_rgb().into()),
        )
    }

    fn set_colors(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(Value::Object(object)) = value {
            let num_colors = (object.length(activation)? as usize).min(MAX_COLORS);
            self.0.write(activation.context.gc_context).num_colors = num_colors;
            for i in 0..num_colors {
                let rgb = object
                    .get_element(activation, i as i32)
                    .coerce_to_u32(activation)?;
                let alpha = self.0.read().colors[i].a;
                self.0.write(activation.context.gc_context).colors[i] = Color::from_rgb(rgb, alpha);
            }
        }
        Ok(())
    }

    fn alphas(&self, context: &mut UpdateContext<'_, 'gc>) -> ArrayObject<'gc> {
        let read = self.0.read();
        ArrayObject::new(
            context.gc_context,
            context.avm1.prototypes().array,
            read.colors[..read.num_colors]
                .iter()
                .map(|c| (f64::from(c.a) / 255.0).into()),
        )
    }

    fn set_alphas(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(Value::Object(object)) = value {
            let num_colors = self.0.read().num_colors;
            for i in 0..num_colors {
                let alpha = if i < object.length(activation)? as usize {
                    let alpha = object
                        .get_element(activation, i as i32)
                        .coerce_to_f64(activation)?;
                    if alpha.is_finite() {
                        (255.0 * alpha + 0.5) as u8
                    } else {
                        u8::MAX
                    }
                } else {
                    u8::MAX
                };
                self.0.write(activation.context.gc_context).colors[i].a = alpha;
            }
        }
        Ok(())
    }

    fn ratios(&self, context: &mut UpdateContext<'_, 'gc>) -> ArrayObject<'gc> {
        ArrayObject::new(
            context.gc_context,
            context.avm1.prototypes().array,
            self.0.read().ratios.iter().map(|&x| x.into()),
        )
    }

    fn set_ratios(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(Value::Object(object)) = value {
            let num_colors = self
                .0
                .read()
                .num_colors
                .min(object.length(activation)? as usize);
            self.0.write(activation.context.gc_context).num_colors = num_colors;
            let ratios: Result<Vec<_>, Error<'gc>> = (0..num_colors)
                .map(|i| {
                    Ok(object
                        .get_element(activation, i as i32)
                        .coerce_to_i32(activation)?
                        .clamp(0, u8::MAX.into()) as u8)
                })
                .collect();
            self.0.write(activation.context.gc_context).ratios = ratios?;
        }
        Ok(())
    }

    fn blur_x(&self) -> f64 {
        self.0.read().blur_x
    }

    fn set_blur_x(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(value) = value {
            let blur_x = value.coerce_to_f64(activation)?.clamp_also_nan(0.0, 255.0);
            self.0.write(activation.context.gc_context).blur_x = blur_x;
        }
        Ok(())
    }

    fn blur_y(&self) -> f64 {
        self.0.read().blur_y
    }

    fn set_blur_y(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(value) = value {
            let blur_y = value.coerce_to_f64(activation)?.clamp_also_nan(0.0, 255.0);
            self.0.write(activation.context.gc_context).blur_y = blur_y;
        }
        Ok(())
    }

    fn strength(&self) -> f64 {
        f64::from(self.0.read().strength) / 256.0
    }

    fn set_strength(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(value) = value {
            let strength = ((value.coerce_to_f64(activation)? * 256.0) as u16).clamp(0, 0xFF00);
            self.0.write(activation.context.gc_context).strength = strength;
        }
        Ok(())
    }

    fn quality(&self) -> i32 {
        self.0.read().quality
    }

    fn set_quality(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(value) = value {
            let quality = value.coerce_to_i32(activation)?.clamp(0, 15);
            self.0.write(activation.context.gc_context).quality = quality;
        }
        Ok(())
    }

    fn type_(&self) -> BevelFilterType {
        self.0.read().type_
    }

    fn set_type(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(value) = value {
            let type_ = value.coerce_to_string(activation)?.as_wstr().into();
            self.0.write(activation.context.gc_context).type_ = type_;
        }
        Ok(())
    }

    fn knockout(&self) -> bool {
        self.0.read().knockout
    }

    fn set_knockout(
        &self,
        activation: &mut Activation<'_, 'gc>,
        value: Option<&Value<'gc>>,
    ) -> Result<(), Error<'gc>> {
        if let Some(value) = value {
            let knockout = value.as_bool(activation.swf_version());
            self.0.write(activation.context.gc_context).knockout = knockout;
        }
        Ok(())
    }
}

macro_rules! gradient_filter_method {
    ($index:literal) => {
        |activation, this, args| method(activation, this, args, $index)
    };
}

const PROTO_DECLS: &[Declaration] = declare_properties! {
    "distance" => property(gradient_filter_method!(1), gradient_filter_method!(2); VERSION_8);
    "angle" => property(gradient_filter_method!(3), gradient_filter_method!(4); VERSION_8);
    "colors" => property(gradient_filter_method!(5), gradient_filter_method!(6); VERSION_8);
    "alphas" => property(gradient_filter_method!(7), gradient_filter_method!(8); VERSION_8);
    "ratios" => property(gradient_filter_method!(9), gradient_filter_method!(10); VERSION_8);
    "blurX" => property(gradient_filter_method!(11), gradient_filter_method!(12); VERSION_8);
    "blurY" => property(gradient_filter_method!(13), gradient_filter_method!(14); VERSION_8);
    "quality" => property(gradient_filter_method!(15), gradient_filter_method!(16); VERSION_8);
    "strength" => property(gradient_filter_method!(17), gradient_filter_method!(18); VERSION_8);
    "knockout" => property(gradient_filter_method!(19), gradient_filter_method!(20); VERSION_8);
    "type" => property(gradient_filter_method!(21), gradient_filter_method!(22); VERSION_8);
};

fn method<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Object<'gc>,
    args: &[Value<'gc>],
    index: u16,
) -> Result<Value<'gc>, Error<'gc>> {
    const GLOW_CONSTRUCTOR: u16 = 0;
    const GET_DISTANCE: u16 = 1;
    const SET_DISTANCE: u16 = 2;
    const GET_ANGLE: u16 = 3;
    const SET_ANGLE: u16 = 4;
    const GET_COLORS: u16 = 5;
    const SET_COLORS: u16 = 6;
    const GET_ALPHAS: u16 = 7;
    const SET_ALPHAS: u16 = 8;
    const GET_RATIOS: u16 = 9;
    const SET_RATIOS: u16 = 10;
    const GET_BLUR_X: u16 = 11;
    const SET_BLUR_X: u16 = 12;
    const GET_BLUR_Y: u16 = 13;
    const SET_BLUR_Y: u16 = 14;
    const GET_QUALITY: u16 = 15;
    const SET_QUALITY: u16 = 16;
    const GET_STRENGTH: u16 = 17;
    const SET_STRENGTH: u16 = 18;
    const GET_KNOCKOUT: u16 = 19;
    const SET_KNOCKOUT: u16 = 20;
    const GET_TYPE: u16 = 21;
    const SET_TYPE: u16 = 22;
    const BEVEL_CONSTRUCTOR: u16 = 1000;

    if index == BEVEL_CONSTRUCTOR {
        let gradient_bevel_filter = GradientFilter::new(activation, args)?;
        this.set_native(
            activation.context.gc_context,
            NativeObject::GradientBevelFilter(gradient_bevel_filter),
        );
        return Ok(this.into());
    }
    if index == GLOW_CONSTRUCTOR {
        let gradient_glow_filter = GradientFilter::new(activation, args)?;
        this.set_native(
            activation.context.gc_context,
            NativeObject::GradientGlowFilter(gradient_glow_filter),
        );
        return Ok(this.into());
    }

    let this = match this.native() {
        NativeObject::GradientBevelFilter(gradient_bevel_filter) => gradient_bevel_filter,
        NativeObject::GradientGlowFilter(gradient_glow_filter) => gradient_glow_filter,
        _ => return Ok(Value::Undefined),
    };

    Ok(match index {
        GET_DISTANCE => this.distance().into(),
        SET_DISTANCE => {
            this.set_distance(activation, args.get(0))?;
            Value::Undefined
        }
        GET_ANGLE => this.angle().into(),
        SET_ANGLE => {
            this.set_angle(activation, args.get(0))?;
            Value::Undefined
        }
        GET_COLORS => this.colors(&mut activation.context).into(),
        SET_COLORS => {
            this.set_colors(activation, args.get(0))?;
            Value::Undefined
        }
        GET_ALPHAS => this.alphas(&mut activation.context).into(),
        SET_ALPHAS => {
            this.set_alphas(activation, args.get(0))?;
            Value::Undefined
        }
        GET_RATIOS => this.ratios(&mut activation.context).into(),
        SET_RATIOS => {
            this.set_ratios(activation, args.get(0))?;
            Value::Undefined
        }
        GET_BLUR_X => this.blur_x().into(),
        SET_BLUR_X => {
            this.set_blur_x(activation, args.get(0))?;
            Value::Undefined
        }
        GET_BLUR_Y => this.blur_y().into(),
        SET_BLUR_Y => {
            this.set_blur_y(activation, args.get(0))?;
            Value::Undefined
        }
        GET_QUALITY => this.quality().into(),
        SET_QUALITY => {
            this.set_quality(activation, args.get(0))?;
            Value::Undefined
        }
        GET_STRENGTH => this.strength().into(),
        SET_STRENGTH => {
            this.set_strength(activation, args.get(0))?;
            Value::Undefined
        }
        GET_KNOCKOUT => this.knockout().into(),
        SET_KNOCKOUT => {
            this.set_knockout(activation, args.get(0))?;
            Value::Undefined
        }
        GET_TYPE => {
            let type_: &WStr = this.type_().into();
            AvmString::from(type_).into()
        }
        SET_TYPE => {
            this.set_type(activation, args.get(0))?;
            Value::Undefined
        }
        _ => Value::Undefined,
    })
}

pub fn create_bevel_constructor<'gc>(
    context: &mut GcContext<'_, 'gc>,
    proto: Object<'gc>,
    fn_proto: Object<'gc>,
) -> Object<'gc> {
    let gradient_bevel_filter_proto = ScriptObject::new(context.gc_context, Some(proto));
    define_properties_on(PROTO_DECLS, context, gradient_bevel_filter_proto, fn_proto);
    FunctionObject::constructor(
        context.gc_context,
        Executable::Native(gradient_filter_method!(1000)),
        constructor_to_fn!(gradient_filter_method!(1000)),
        fn_proto,
        gradient_bevel_filter_proto.into(),
    )
}

pub fn create_glow_constructor<'gc>(
    context: &mut GcContext<'_, 'gc>,
    proto: Object<'gc>,
    fn_proto: Object<'gc>,
) -> Object<'gc> {
    let gradient_bevel_filter_proto = ScriptObject::new(context.gc_context, Some(proto));
    define_properties_on(PROTO_DECLS, context, gradient_bevel_filter_proto, fn_proto);
    FunctionObject::constructor(
        context.gc_context,
        Executable::Native(gradient_filter_method!(0)),
        constructor_to_fn!(gradient_filter_method!(0)),
        fn_proto,
        gradient_bevel_filter_proto.into(),
    )
}
