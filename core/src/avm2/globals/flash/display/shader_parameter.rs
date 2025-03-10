use crate::{
    avm2::{string::AvmString, Activation, Error, Multiname, TObject, Value},
    pixel_bender::PixelBenderParam,
};

pub fn make_shader_parameter<'gc>(
    activation: &mut Activation<'_, 'gc>,
    param: PixelBenderParam,
    index: usize,
) -> Result<Value<'gc>, Error<'gc>> {
    let ns = activation.avm2().flash_display_internal;

    match param {
        PixelBenderParam::Normal {
            name,
            param_type,
            metadata,
            ..
        } => {
            let mut obj = activation
                .avm2()
                .classes()
                .shaderparameter
                .construct(activation, &[])?;
            let type_name =
                AvmString::new_utf8(activation.context.gc_context, &param_type.to_string());

            obj.set_property(&Multiname::new(ns, "_index"), index.into(), activation)?;
            obj.set_property(&Multiname::new(ns, "_type"), type_name.into(), activation)?;
            for meta in metadata {
                let name = AvmString::new_utf8(activation.context.gc_context, &meta.key);
                let value = meta.value.clone().into_avm2_value(activation)?;
                obj.set_public_property(name, value, activation)?;
            }
            obj.set_public_property(
                "name",
                AvmString::new_utf8(activation.context.gc_context, name).into(),
                activation,
            )?;
            Ok(obj.into())
        }
        PixelBenderParam::Texture { name, channels, .. } => {
            let mut obj = activation
                .avm2()
                .classes()
                .shaderinput
                .construct(activation, &[])?;
            obj.set_property(
                &Multiname::new(ns, "_channels"),
                channels.into(),
                activation,
            )?;
            obj.set_property(&Multiname::new(ns, "_index"), index.into(), activation)?;
            obj.set_public_property(
                "name",
                AvmString::new_utf8(activation.context.gc_context, name).into(),
                activation,
            )?;
            Ok(obj.into())
        }
    }
}
