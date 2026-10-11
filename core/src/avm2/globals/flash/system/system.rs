//! `flash.system.System` native methods

use crate::avm2::Error;
use crate::avm2::activation::Activation;
use crate::avm2::error::{make_error_2017, make_error_2018};
use crate::avm2::function::FunctionArgs;
use crate::avm2::parameters::ParametersExt;
use crate::avm2::value::Value;
use crate::avm2_stub_method;
use ruffle_common::sandbox::SandboxType;

/// Implements `flash.system.System.setClipboard` method
pub fn set_clipboard<'gc>(
    activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    // The following restrictions only apply to the plugin.
    // TODO: Check the type of event that triggered the function call.
    #[cfg(target_family = "wasm")]
    if false {
        return Err(crate::avm2::error::make_error_2176(activation));
    }

    let new_content = args.get_string_non_null(activation, 0, "text")?;
    activation
        .context
        .ui
        .set_clipboard_content(new_content.to_string());

    Ok(Value::Undefined)
}

pub fn exit<'gc>(
    activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    avm2_stub_method!(activation, "flash.system.System", "exit");

    if cfg!(target_family = "wasm") {
        return Err(make_error_2018(activation));
    }

    let movie = activation.caller_movie_or_root();
    if !matches!(movie.sandbox_type(), SandboxType::LocalTrusted) {
        return Err(make_error_2017(activation));
    }

    // TODO: Actually quit.
    Err("Unsupported System.exit() was called".into())
}

/// Bytes em uso no heap do Ruffle (alocador contador do ruffle_web); sem ele, 90 MB como antes.
fn memoria_em_uso() -> f64 {
    crate::memoria_n7::contar(&crate::memoria_n7::LEITURAS_TOTAL_MEMORY);
    let vivo = crate::memoria_n7::heap_vivo();
    if vivo > 0 {
        vivo as f64
    } else {
        (1024 * 1024 * 90) as f64
    }
}

/// Memoria reservada pelo processo (no wasm: tamanho da memoria linear).
fn memoria_reservada() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        (core::arch::wasm32::memory_size(0) * 65536) as f64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        memoria_em_uso().max((1024 * 1024 * 100) as f64)
    }
}

/// Implements `flash.system.System.totalMemoryNumber`
pub fn get_total_memory_number<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(memoria_em_uso().into())
}

/// Implements `flash.system.System.privateMemory`
pub fn get_private_memory<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(memoria_reservada().max(memoria_em_uso()).into())
}

/// Implements `flash.system.System.freeMemory`
pub fn get_free_memory<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    _this: Value<'gc>,
    _args: FunctionArgs<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    Ok((memoria_reservada() - memoria_em_uso()).max(0.0).into())
}
