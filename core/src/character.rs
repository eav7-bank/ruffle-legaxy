use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Weak};

use crate::bitmap::bitmap_data::{Color, TexturaCompartilhada};

use crate::backend::audio::SoundHandle;
use crate::binary_data::BinaryData;
use crate::display_object::{
    Avm1Button, Avm2Button, BitmapClass, EditText, Graphic, MorphShape, MovieClip, Text, Video,
};
use crate::font::Font;
use gc_arena::barrier::unlock;
use gc_arena::lock::Lock;
use gc_arena::{Collect, Gc, Mutation};
use ruffle_render::backend::RenderBackend;
use ruffle_render::bitmap::{Bitmap as RenderBitmap, BitmapHandle, BitmapSize};
use ruffle_render::error::Error as RenderError;
use swf::DefineBitsLossless;

#[derive(Copy, Clone, Collect, Debug)]
#[collect(no_drop)]
pub enum Character<'gc> {
    EditText(EditText<'gc>),
    Graphic(Graphic<'gc>),
    MovieClip(MovieClip<'gc>),
    Bitmap(Gc<'gc, BitmapCharacter<'gc>>),
    Avm1Button(Avm1Button<'gc>),
    Avm2Button(Avm2Button<'gc>),
    Font(Font<'gc>),
    MorphShape(MorphShape<'gc>),
    Text(Text<'gc>),
    Sound(#[collect(require_static)] SoundHandle),
    Video(Video<'gc>),
    BinaryData(Gc<'gc, BinaryData>),
}

#[derive(Collect, Debug)]
#[collect(no_drop)]
pub struct BitmapCharacter<'gc> {
    /// Imagem compactada + cache fraco dos pixels decodificados (compartilhado tambem com os
    /// `BitmapData` de decodificacao preguicosa, ver [`FonteDoSimbolo`]).
    #[collect(require_static)]
    fonte: Rc<FonteDoSimbolo>,
    /// A lazily constructed GPU handle, used when performing fills with this bitmap
    #[collect(require_static)]
    handle: OnceCell<BitmapHandle>,
    /// The bitmap class set by `SymbolClass` - this is used when we instantaite
    /// a `Bitmap` displayobject.
    avm2_class: Lock<BitmapClass<'gc>>,
}

impl<'gc> BitmapCharacter<'gc> {
    pub fn new(compressed: CompressedBitmap) -> Self {
        Self {
            fonte: Rc::new(FonteDoSimbolo {
                compressed,
                compartilhado: RefCell::new(PixelsDoSimbolo::default()),
            }),
            handle: OnceCell::default(),
            avm2_class: Lock::new(BitmapClass::NoSubclass),
        }
    }

    pub fn compressed(&self) -> &CompressedBitmap {
        &self.fonte.compressed
    }

    /// Fonte do simbolo, para um `BitmapData` decodificar so quando precisar dos pixels.
    pub fn fonte(&self) -> Rc<FonteDoSimbolo> {
        self.fonte.clone()
    }

    /// Pixels decodificados deste simbolo, compartilhados entre as instancias vivas (decodifica
    /// so quando nao ha nenhuma viva).
    pub fn pixels_compartilhados(&self) -> Result<SimboloDecodificado, RenderError> {
        self.fonte.pixels_compartilhados()
    }
}

/// Imagem compactada de um simbolo de bitmap e o cache fraco dos seus pixels decodificados.
///
/// Naruto Legaxy: como no Flash, a imagem embutida so e decodificada quando os pixels sao usados
/// de verdade. O jogo cria centenas de `new ClasseDoQuadro()` ao montar uma luta e so desenha
/// parte deles; decodificar todos na criacao segurava ~250 MB durante o carregamento (o iPhone
/// mata a aba). O `BitmapData` guarda um `Rc` desta fonte e decodifica no primeiro acesso.
pub struct FonteDoSimbolo {
    compressed: CompressedBitmap,
    compartilhado: RefCell<PixelsDoSimbolo>,
}

impl std::fmt::Debug for FonteDoSimbolo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FonteDoSimbolo").finish()
    }
}

impl FonteDoSimbolo {
    pub fn size(&self) -> BitmapSize {
        self.compressed.size()
    }

    /// Textura da GPU do simbolo, se alguma instancia viva ja a registrou (desenhar sem decodificar).
    pub fn textura_registrada(&self) -> Option<Arc<TexturaCompartilhada>> {
        let c = self.compartilhado.borrow();
        c.textura.upgrade().filter(|t| t.registrada())
    }

    /// Pixels decodificados deste simbolo, compartilhados entre as instancias vivas (decodifica
    /// so quando nao ha nenhuma viva).
    pub fn pixels_compartilhados(&self) -> Result<SimboloDecodificado, RenderError> {
        let mut c = self.compartilhado.borrow_mut();
        if let Some(pixels) = c.pixels.upgrade() {
            let textura = match c.textura.upgrade() {
                Some(t) => t,
                None => {
                    let t = Arc::new(TexturaCompartilhada::default());
                    c.textura = Arc::downgrade(&t);
                    t
                }
            };
            crate::memoria_n7::contar(&crate::memoria_n7::SIMBOLOS_REAPROVEITADOS);
            return Ok(SimboloDecodificado {
                pixels,
                textura,
                width: c.width,
                height: c.height,
                transparente: c.transparente,
                decodificou: false,
            });
        }
        let _etq = crate::memoria_n7::Etiqueta::nova(4);
        // JPEG sem alfa: direto no buffer final (ver bitmap::jpeg_n7); resto: caminho normal
        let direto = match &self.compressed {
            CompressedBitmap::Jpeg {
                data, alpha: None, ..
            } => crate::bitmap::jpeg_n7::decodificar_direto(data),
            _ => None,
        };
        let (width, height, transparente, pixels) = match direto {
            Some((w, h, px)) => (w, h, false, Arc::new(px)),
            None => {
                let decoded = self.compressed.decode()?;
                let transparente =
                    !matches!(decoded.format(), ruffle_render::bitmap::BitmapFormat::Rgb);
                let (w, h) = (decoded.width(), decoded.height());
                let px: Vec<Color> = decoded.as_colors().map(Color::from).collect();
                (w, h, transparente, Arc::new(px))
            }
        };
        let textura = Arc::new(TexturaCompartilhada::default());
        *c = PixelsDoSimbolo {
            pixels: Arc::downgrade(&pixels),
            textura: Arc::downgrade(&textura),
            width,
            height,
            transparente,
        };
        crate::memoria_n7::contar(&crate::memoria_n7::SIMBOLOS_DECODIFICADOS);
        Ok(SimboloDecodificado {
            pixels,
            textura,
            width,
            height,
            transparente,
            decodificou: true,
        })
    }
}

impl<'gc> BitmapCharacter<'gc> {
    pub fn avm2_class(&self) -> BitmapClass<'gc> {
        self.avm2_class.get()
    }

    pub fn set_avm2_class(this: Gc<'gc, Self>, bitmap_class: BitmapClass<'gc>, mc: &Mutation<'gc>) {
        unlock!(Gc::write(mc, this), Self, avm2_class).set(bitmap_class);
    }

    pub fn bitmap_handle(
        &self,
        backend: &mut dyn RenderBackend,
    ) -> Result<BitmapHandle, RenderError> {
        // FIXME - use `OnceCell::get_or_try_init` when stabilized.
        if let Some(handle) = self.handle.get() {
            return Ok(handle.clone());
        }
        let decoded = self.fonte.compressed.decode()?;
        let new_handle = backend.register_bitmap(decoded)?;
        // FIXME - do we ever want to release this handle, to avoid taking up GPU memory?
        self.handle.set(new_handle.clone()).unwrap();
        Ok(new_handle)
    }
}

/// Cache fraco dos pixels decodificados de um simbolo de bitmap.
#[derive(Default)]
struct PixelsDoSimbolo {
    pixels: Weak<Vec<Color>>,
    textura: Weak<TexturaCompartilhada>,
    width: u32,
    height: u32,
    transparente: bool,
}

impl std::fmt::Debug for PixelsDoSimbolo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PixelsDoSimbolo")
            .field("vivo", &(self.pixels.strong_count() > 0))
            .finish()
    }
}

/// Pixels de um simbolo prontos para uma instancia (ver `BitmapCharacter::pixels_compartilhados`).
pub struct SimboloDecodificado {
    pub pixels: Arc<Vec<Color>>,
    pub textura: Arc<TexturaCompartilhada>,
    pub width: u32,
    pub height: u32,
    pub transparente: bool,
    pub decodificou: bool,
}

/// Holds a bitmap from an SWF tag, plus the decoded width/height.
/// We avoid decompressing the image until it's actually needed - some pathological SWFS
/// like 'House' have thousands of highly-compressed (mostly empty) bitmaps, which can
/// take over 10GB of ram if we decompress them all during preloading.
#[derive(Clone, Debug)]
pub enum CompressedBitmap {
    Jpeg {
        data: Vec<u8>,
        alpha: Option<Vec<u8>>,
        width: u32,
        height: u32,
    },
    Lossless(DefineBitsLossless<'static>),
}

impl CompressedBitmap {
    pub fn size(&self) -> BitmapSize {
        match self {
            CompressedBitmap::Jpeg { width, height, .. } => BitmapSize {
                width: *width,
                height: *height,
            },
            CompressedBitmap::Lossless(define_bits_lossless) => BitmapSize {
                width: define_bits_lossless.width.into(),
                height: define_bits_lossless.height.into(),
            },
        }
    }
    pub fn decode(&self) -> Result<RenderBitmap<'static>, RenderError> {
        match self {
            CompressedBitmap::Jpeg {
                data,
                alpha,
                width: _,
                height: _,
            } => ruffle_render::utils::decode_define_bits_jpeg(data, alpha.as_deref()),
            CompressedBitmap::Lossless(define_bits_lossless) => {
                ruffle_render::utils::decode_define_bits_lossless(define_bits_lossless)
            }
        }
    }
}
