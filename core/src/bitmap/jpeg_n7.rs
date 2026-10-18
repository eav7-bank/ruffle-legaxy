//! Decodificacao de JPEG direto no buffer final (Naruto Legaxy).
//!
//! O caminho normal (`jpeg-decoder`) devolve RGB em um `Vec` proprio, que depois vira `Vec<Color>`:
//! por um instante a imagem existe em planos YCbCr + RGB + RGBA (~7 bytes por pixel). O jogo
//! carrega tiras de animacao de ate 27900x1664 (185 MB em RGBA): so essa conversao punha ~140 MB
//! a mais no pico, e a memoria do wasm nunca encolhe (o iPhone mata a aba). O `zune-jpeg`
//! decodifica em faixas e escreve RGBA direto no `Vec<Color>` final (~4 bytes por pixel).
//!
//! Devolve `None` (o chamador usa o caminho normal) para o que nao for JPEG YCbCr/RGB comum:
//! cinza, CMYK, PNG/GIF, erro.

use crate::bitmap::bitmap_data::Color;
use ruffle_render::utils::{JpegTagFormat, determine_jpeg_tag_format, remove_invalid_jpeg_data};
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

/// Pixels opacos (alfa 255) de um JPEG sem canal alfa separado: `(largura, altura, pixels)`.
pub fn decodificar_direto(data: &[u8]) -> Option<(u32, u32, Vec<Color>)> {
    if determine_jpeg_tag_format(data) != JpegTagFormat::Jpeg {
        return None;
    }
    let data = remove_invalid_jpeg_data(data);
    let opcoes = DecoderOptions::default()
        .jpeg_set_out_colorspace(ColorSpace::RGBA)
        .set_max_width(u16::MAX as usize)
        .set_max_height(u16::MAX as usize);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(&data[..]), opcoes);
    decoder.decode_headers().ok()?;
    if !matches!(decoder.input_colorspace()?, ColorSpace::YCbCr | ColorSpace::RGB) {
        return None;
    }
    if decoder.output_colorspace()? != ColorSpace::RGBA {
        return None;
    }
    let (w, h) = decoder.dimensions()?;
    let n = w.checked_mul(h)?;
    if n == 0 {
        return None;
    }
    let mut pixels = vec![Color::default(); n];
    // SAFETY: `Color` e `#[repr(C, align(4))]` com 4 campos `u8` (r, g, b, a) e qualquer padrao
    // de bits e valido; a fatia cobre exatamente os `n * 4` bytes do `Vec`.
    let bytes = unsafe { std::slice::from_raw_parts_mut(pixels.as_mut_ptr().cast::<u8>(), n * 4) };
    if let Err(e) = decoder.decode_into(bytes) {
        tracing::warn!("JPEG direto falhou ({e:?}); usando o decodificador normal");
        return None;
    }
    Some((w as u32, h as u32, pixels))
}
