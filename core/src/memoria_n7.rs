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

/// Bytes vivos por tamanho de bloco: [< 4 KiB, 4 KiB..1 MiB, >= 1 MiB] (objetos AS3 e estruturas
/// pequenas x buffers medios x pixels/arquivos grandes) e quantos blocos >= 1 MiB estao vivos.
pub static HEAP_CLASSE: [AtomicUsize; 3] = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
pub static BLOCOS_GRANDES: AtomicUsize = AtomicUsize::new(0);

#[inline]
fn classe(n: usize) -> usize {
    if n < 4096 {
        0
    } else if n < 1024 * 1024 {
        1
    } else {
        2
    }
}

#[inline]
pub fn somar_heap(n: usize) {
    let c = classe(n);
    HEAP_CLASSE[c].fetch_add(n, Ordering::Relaxed);
    if c == 2 {
        BLOCOS_GRANDES.fetch_add(1, Ordering::Relaxed);
    }
    let v = HEAP_VIVO.fetch_add(n, Ordering::Relaxed) + n;
    if v > HEAP_PICO.load(Ordering::Relaxed) {
        HEAP_PICO.store(v, Ordering::Relaxed);
    }
}

#[inline]
pub fn subtrair_heap(n: usize) {
    let c = classe(n);
    HEAP_CLASSE[c].fetch_sub(n, Ordering::Relaxed);
    if c == 2 {
        BLOCOS_GRANDES.fetch_sub(1, Ordering::Relaxed);
    }
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
            if l.size() >= 1024 * 1024 {
                grande_alocado(p as usize, l.size());
            }
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc_zeroed(l) };
        if p.is_null() {
            ALOC_FALHOU.store(l.size(), Ordering::Relaxed);
        } else {
            somar_heap(l.size());
            if l.size() >= 1024 * 1024 {
                grande_alocado(p as usize, l.size());
            }
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        if l.size() >= 1024 * 1024 {
            grande_liberado(p as usize);
        }
        unsafe { std::alloc::System.dealloc(p, l) };
        subtrair_heap(l.size());
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, novo: usize) -> *mut u8 {
        let q = unsafe { std::alloc::System.realloc(p, l, novo) };
        if q.is_null() {
            ALOC_FALHOU.store(novo, Ordering::Relaxed);
        } else {
            if l.size() >= 1024 * 1024 {
                grande_liberado(p as usize);
            }
            subtrair_heap(l.size());
            somar_heap(novo);
            if novo >= 1024 * 1024 {
                grande_alocado(q as usize, novo);
            }
        }
        q
    }
}

/// Multiplicador aplicado ao que o jogo le em `System.totalMemory` (bits de f64; padrao 1,0). No
/// celular o app pode subir (ex.: 2,0) para o jogo liberar recursos com metade da memoria real.
pub static FATOR_TOTAL_MEMORY: AtomicU64 = AtomicU64::new(0x3FF0000000000000);

#[inline]
pub fn fator_total_memory() -> f64 {
    f64::from_bits(FATOR_TOTAL_MEMORY.load(Ordering::Relaxed))
}

pub fn definir_fator_total_memory(f: f64) {
    if f.is_finite() && f > 0.0 {
        FATOR_TOTAL_MEMORY.store(f.to_bits(), Ordering::Relaxed);
    }
}

/// Quadros restantes de "pressao de memoria": enquanto > 0, `System.totalMemory` informa no minimo
/// [`MEMORIA_SOB_PRESSAO`] para o jogo soltar tudo o que ele libera por memoria na proxima troca de
/// tela/luta (aviso de pouca memoria do sistema).
pub static QUADROS_SOB_PRESSAO: AtomicU64 = AtomicU64::new(0);
/// 1 GiB: acima do limite do Naruto Legaxy (700 * 1024 * 1000 bytes).
pub const MEMORIA_SOB_PRESSAO: f64 = 1024.0 * 1024.0 * 1024.0;

/// O sistema avisou pouca memoria: o jogo passa a ver memoria alta por `quadros` quadros.
pub fn sinalizar_pressao(quadros: u64) {
    QUADROS_SOB_PRESSAO.fetch_max(quadros, Ordering::Relaxed);
}

/// Chamado a cada quadro executado.
#[inline]
pub fn passar_quadro_pressao() {
    let _ = QUADROS_SOB_PRESSAO.try_update(Ordering::Relaxed, Ordering::Relaxed, |q| q.checked_sub(1));
}

/// Avisos de pouca memoria recebidos (`Player::liberar_memoria`).
pub static LIBERACOES: AtomicU64 = AtomicU64::new(0);

/// Bytes que o jogo "usa" segundo `System.totalMemory`: heap vivo + texturas da GPU, vezes o fator;
/// `None` sem alocador contador (o chamador usa o valor fixo antigo).
pub fn memoria_do_jogo() -> Option<f64> {
    let vivo = heap_vivo();
    if vivo == 0 {
        return None;
    }
    let gpu = ruffle_render::memoria::bytes_texturas() as f64;
    let mut v = (vivo as f64 + gpu) * fator_total_memory();
    if QUADROS_SOB_PRESSAO.load(Ordering::Relaxed) > 0 {
        v = v.max(MEMORIA_SOB_PRESSAO);
    }
    Some(v)
}

/// Objetos vivos no coletor (gc-arena), atualizado a cada quadro executado.
pub static OBJETOS_GC: AtomicU64 = AtomicU64::new(0);

/// Bytes (capacidade) dos `ByteArray` vivos e quantos estao vivos.
pub static BYTEARRAY_BYTES: AtomicUsize = AtomicUsize::new(0);

#[inline]
pub fn ajustar_bytearray(antes: usize, depois: usize) {
    if depois > antes {
        BYTEARRAY_BYTES.fetch_add(depois - antes, Ordering::Relaxed);
    } else if antes > depois {
        BYTEARRAY_BYTES.fetch_sub(antes - depois, Ordering::Relaxed);
    }
}

// ===== Diagnostico: quem segura os blocos grandes (>= 1 MiB) =====
// Etiqueta corrente (o wasm e de uma thread so): os blocos grandes alocados enquanto ela vale
// ficam contados nela ate serem liberados. 0 = outros, 1 = imagem do Loader (loadBytes),
// 2 = SWF carregado, 3 = BitmapData, 5 = render, 6 = quadro (AS3/timeline).
pub static ETIQUETA: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub static GRANDES_POR_ETIQUETA: [AtomicUsize; 8] = [
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
];
const MAX_GRANDES: usize = 4096;
static TRAVA_GRANDES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static mut GRANDES: [(usize, usize, u8); MAX_GRANDES] = [(0, 0, 0); MAX_GRANDES];

pub struct Etiqueta(u8);
impl Etiqueta {
    #[inline]
    pub fn nova(e: u8) -> Self {
        Etiqueta(ETIQUETA.swap(e, Ordering::Relaxed))
    }
}
impl Drop for Etiqueta {
    #[inline]
    fn drop(&mut self) {
        ETIQUETA.store(self.0, Ordering::Relaxed);
    }
}

fn travar() {
    while TRAVA_GRANDES
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::hint::spin_loop();
    }
}

#[allow(static_mut_refs)]
fn grande_alocado(p: usize, n: usize) {
    let e = ETIQUETA.load(Ordering::Relaxed);
    travar();
    unsafe {
        for slot in GRANDES.iter_mut() {
            if slot.0 == 0 {
                *slot = (p, n, e);
                GRANDES_POR_ETIQUETA[(e & 7) as usize].fetch_add(n, Ordering::Relaxed);
                break;
            }
        }
    }
    TRAVA_GRANDES.store(false, Ordering::Release);
}

#[allow(static_mut_refs)]
fn grande_liberado(p: usize) {
    travar();
    unsafe {
        for slot in GRANDES.iter_mut() {
            if slot.0 == p {
                GRANDES_POR_ETIQUETA[(slot.2 & 7) as usize].fetch_sub(slot.1, Ordering::Relaxed);
                *slot = (0, 0, 0);
                break;
            }
        }
    }
    TRAVA_GRANDES.store(false, Ordering::Release);
}

