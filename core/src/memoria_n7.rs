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

/// Divida de GC por quadro, em fracao do numero de objetos vivos (bits de f64). Padrao 0,004
/// (~1 ciclo completo a cada 10 s a 30 fps). 0 desliga.
pub static FATOR_COLETA_POR_QUADRO: AtomicU64 = AtomicU64::new(0x3F70624DD2F1A9FC);
/// Leituras de `System.totalMemory` pelo conteudo.
pub static LEITURAS_TOTAL_MEMORY: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn fator_coleta_por_quadro() -> f64 {
    f64::from_bits(FATOR_COLETA_POR_QUADRO.load(Ordering::Relaxed))
}

/// Quadros do SWF executados (`Player::run_frame`): fps real do conteudo.
pub static QUADROS_EXECUTADOS: AtomicU64 = AtomicU64::new(0);

// ===== Ruffle Legaxy mobile (fase 1): alocador contador, memoria da GPU e pressao de memoria =====

/// Alocador global que soma/subtrai cada alocacao em [`HEAP_VIVO`]. O frontend declara
/// `#[global_allocator] static A: ruffle_core::memoria_n7::AlocadorContador = AlocadorContador;`
/// (web, app Android, desktop). Sem ele o heap fica em 0 e `System.totalMemory` cai no valor fixo
/// antigo (90 MB) — e o jogo NUNCA libera os recursos de luta, que ele so solta acima de 700 MB.
pub struct AlocadorContador;

unsafe impl std::alloc::GlobalAlloc for AlocadorContador {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc(l) };
        if p.is_null() {
            ALOC_FALHOU.store(l.size(), Ordering::Relaxed);
        } else {
            somar_heap(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc_zeroed(l) };
        if p.is_null() {
            ALOC_FALHOU.store(l.size(), Ordering::Relaxed);
        } else {
            somar_heap(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(p, l) };
        subtrair_heap(l.size());
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, novo: usize) -> *mut u8 {
        let q = unsafe { std::alloc::System.realloc(p, l, novo) };
        if q.is_null() {
            ALOC_FALHOU.store(novo, Ordering::Relaxed);
        } else {
            subtrair_heap(l.size());
            somar_heap(novo);
        }
        q
    }
}
