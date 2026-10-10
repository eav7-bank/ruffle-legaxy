//! Medicao de memoria (Naruto Legaxy): contadores baratos (atomicos) lidos pela pagina para
//! achar o que cresce entre lutas, e o "heap vivo" usado em `System.totalMemory`.
//!
//! O heap vivo e preenchido pelo alocador global do `ruffle_web` (que soma/subtrai cada
//! alocacao); em outras plataformas fica em 0 e quem le usa um valor de reserva.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Bytes vivos no alocador (somente quando o alocador contador estiver ativo).
pub static HEAP_VIVO: AtomicUsize = AtomicUsize::new(0);
/// Maior valor de `HEAP_VIVO` ja visto.
pub static HEAP_PICO: AtomicUsize = AtomicUsize::new(0);
/// Tamanho do ultimo pedido de alocacao que falhou (0 = nenhum).
pub static ALOC_FALHOU: AtomicUsize = AtomicUsize::new(0);

/// `BitmapRawData` vivos.
pub static BMP_VIVOS: AtomicU64 = AtomicU64::new(0);
/// Bytes de pixels proprios (nao compartilhados) dos `BitmapRawData` vivos.
pub static BMP_BYTES: AtomicU64 = AtomicU64::new(0);
/// `BitmapRawData` criados desde o inicio.
pub static BMP_CRIADOS: AtomicU64 = AtomicU64::new(0);
/// `BitmapData.dispose()` chamados.
pub static BMP_DISPOSE: AtomicU64 = AtomicU64::new(0);
/// Simbolos de bitmap decodificados (PNG/JPEG -> pixels).
pub static SIMBOLOS_DECODIFICADOS: AtomicU64 = AtomicU64::new(0);
/// Instancias de simbolo que reaproveitaram pixels ja decodificados.
pub static SIMBOLOS_REAPROVEITADOS: AtomicU64 = AtomicU64::new(0);
/// Texturas de GPU registradas para BitmapData.
pub static TEXTURAS_REGISTRADAS: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn somar_heap(n: usize) {
    let v = HEAP_VIVO.fetch_add(n, Ordering::Relaxed) + n;
    if v > HEAP_PICO.load(Ordering::Relaxed) {
        HEAP_PICO.store(v, Ordering::Relaxed);
    }
}

#[inline]
pub fn subtrair_heap(n: usize) {
    HEAP_VIVO.fetch_sub(n, Ordering::Relaxed);
}

#[inline]
pub fn heap_vivo() -> usize {
    HEAP_VIVO.load(Ordering::Relaxed)
}

#[inline]
pub fn contar(c: &AtomicU64) {
    c.fetch_add(1, Ordering::Relaxed);
}

/// Acompanha um `BitmapRawData`: conta vivos e bytes de pixels proprios.
#[derive(Debug)]
pub struct ContadorBitmap(u64);

impl ContadorBitmap {
    pub fn new(bytes: u64) -> Self {
        BMP_VIVOS.fetch_add(1, Ordering::Relaxed);
        BMP_CRIADOS.fetch_add(1, Ordering::Relaxed);
        BMP_BYTES.fetch_add(bytes, Ordering::Relaxed);
        Self(bytes)
    }

    /// Troca os bytes proprios (ex.: pixels compartilhados passaram a ser proprios).
    pub fn definir(&mut self, bytes: u64) {
        BMP_BYTES.fetch_sub(self.0, Ordering::Relaxed);
        BMP_BYTES.fetch_add(bytes, Ordering::Relaxed);
        self.0 = bytes;
    }

    pub fn dispose(&mut self) {
        BMP_DISPOSE.fetch_add(1, Ordering::Relaxed);
        self.definir(0);
    }
}

impl Drop for ContadorBitmap {
    fn drop(&mut self) {
        BMP_VIVOS.fetch_sub(1, Ordering::Relaxed);
        BMP_BYTES.fetch_sub(self.0, Ordering::Relaxed);
    }
}
