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
/// Bytes de pixels novos alocados desde o inicio (BitmapData proprios + simbolos decodificados).
/// Guia a coleta: memoria fora do gc-arena que so volta quando o coletor passa.
pub static BMP_BYTES_NOVOS: AtomicU64 = AtomicU64::new(0);
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

/// Blocos vivos por tamanho (de 16 em 16 bytes ate 4 KiB; o ultimo junta o resto).
pub static HISTOGRAMA: [AtomicUsize; 257] = [const { AtomicUsize::new(0) }; 257];

#[inline]
fn balde(n: usize) -> usize {
    (n / 16).min(256)
}

#[inline]
pub fn somar_heap(n: usize) {
    HISTOGRAMA[balde(n)].fetch_add(1, Ordering::Relaxed);
    let c = classe(n);
    HEAP_CLASSE[c].fetch_add(n, Ordering::Relaxed);
    if c == 2 {
        BLOCOS_GRANDES.fetch_add(1, Ordering::Relaxed);
    }
    let v = HEAP_VIVO.fetch_add(n, Ordering::Relaxed) + n;
    if v > HEAP_PICO.load(Ordering::Relaxed) {
        HEAP_PICO.store(v, Ordering::Relaxed);
        foto_do_pico();
    }
}

#[inline]
pub fn subtrair_heap(n: usize) {
    HISTOGRAMA[balde(n)].fetch_sub(1, Ordering::Relaxed);
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
pub struct ContadorBitmap(std::cell::Cell<u64>);

/// Bytes de pixels proprios por tamanho de imagem: [< 1 MiB, 1..16 MiB, >= 16 MiB] e quantas
/// imagens >= 16 MiB (tiras de animacao) estao vivas.
pub static BMP_CLASSE: [AtomicU64; 3] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
pub static BMP_ENORMES: AtomicU64 = AtomicU64::new(0);

fn bmp_classe(bytes: u64) -> usize {
    if bytes < 1024 * 1024 {
        0
    } else if bytes < 16 * 1024 * 1024 {
        1
    } else {
        2
    }
}

fn bmp_somar(bytes: u64) {
    let c = bmp_classe(bytes);
    BMP_CLASSE[c].fetch_add(bytes, Ordering::Relaxed);
    if c == 2 {
        BMP_ENORMES.fetch_add(1, Ordering::Relaxed);
    }
}

fn bmp_subtrair(bytes: u64) {
    let c = bmp_classe(bytes);
    BMP_CLASSE[c].fetch_sub(bytes, Ordering::Relaxed);
    if c == 2 {
        BMP_ENORMES.fetch_sub(1, Ordering::Relaxed);
    }
}

impl ContadorBitmap {
    pub fn new(bytes: u64) -> Self {
        BMP_VIVOS.fetch_add(1, Ordering::Relaxed);
        BMP_CRIADOS.fetch_add(1, Ordering::Relaxed);
        BMP_BYTES.fetch_add(bytes, Ordering::Relaxed);
        bmp_somar(bytes);
        BMP_BYTES_NOVOS.fetch_add(bytes, Ordering::Relaxed);
        Self(std::cell::Cell::new(bytes))
    }

    /// Troca os bytes proprios (ex.: pixels compartilhados passaram a ser proprios, ou foram
    /// compactados / descompactados).
    pub fn definir(&self, bytes: u64) {
        let antes = self.0.get();
        BMP_BYTES.fetch_sub(antes, Ordering::Relaxed);
        BMP_BYTES.fetch_add(bytes, Ordering::Relaxed);
        bmp_subtrair(antes);
        bmp_somar(bytes);
        if bytes > antes {
            BMP_BYTES_NOVOS.fetch_add(bytes - antes, Ordering::Relaxed);
        }
        self.0.set(bytes);
    }

    pub fn dispose(&self) {
        BMP_DISPOSE.fetch_add(1, Ordering::Relaxed);
        self.definir(0);
    }
}

impl Drop for ContadorBitmap {
    fn drop(&mut self) {
        BMP_VIVOS.fetch_sub(1, Ordering::Relaxed);
        BMP_BYTES.fetch_sub(self.0.get(), Ordering::Relaxed);
        bmp_subtrair(self.0.get());
    }
}

/// Divida de GC por quadro, em fracao do numero de objetos vivos (bits de f64). Padrao 0,001
/// (~1 ciclo completo a cada ~20 s a 30 fps so por ela; a alocacao normal soma mais). 0 desliga.
/// Fase 3: era 0,004 fixo (um ciclo a cada ~5 s, ~10% do tempo da cidade no coletor); agora a
/// parte grande vem de [`divida_coleta_do_quadro`], guiada pelos bytes de bitmap novos.
pub static FATOR_COLETA_POR_QUADRO: AtomicU64 = AtomicU64::new(0x3F50624DD2F1A9FC);

/// Bytes de bitmap novos que valem um ciclo completo extra do coletor.
pub const BYTES_POR_CICLO: f64 = 32.0 * 1024.0 * 1024.0;
/// Teto da divida guiada por bitmap paga por quadro (fracao dos objetos vivos): espalha o ciclo
/// por varios quadros em vez de parar um quadro so quando a luta carrega 100 MB de imagens.
pub const TETO_POR_QUADRO: f64 = 0.01;
/// Fator base enquanto o sistema avisa pouca memoria (o padrao antigo, 0,004).
pub const FATOR_SOB_PRESSAO: f64 = 0.004;
/// Trabalho de um ciclo completo por objeto vivo (marcar + rastrear + manter, `Pacing::DEFAULT`).
const TRABALHO_POR_OBJETO: f64 = 0.55;

static BMP_BYTES_VISTOS: AtomicU64 = AtomicU64::new(0);
/// Divida pendente guiada por bitmap, em fracao dos objetos vivos (bits de f64).
static DIVIDA_BITMAP: AtomicU64 = AtomicU64::new(0);

/// Fracao dos objetos vivos a cobrar como divida do coletor neste quadro: base fixa
/// ([`FATOR_COLETA_POR_QUADRO`]) + parte guiada pelos bytes de bitmap alocados desde o ultimo
/// quadro (um ciclo a cada [`BYTES_POR_CICLO`]), paga aos poucos ([`TETO_POR_QUADRO`]).
pub fn divida_coleta_do_quadro() -> f64 {
    let mut base = fator_coleta_por_quadro();
    if base <= 0.0 {
        return 0.0;
    }
    // Sistema avisou pouca memoria: volta ao ritmo antigo (um ciclo a cada ~5 s) enquanto durar.
    if QUADROS_SOB_PRESSAO.load(Ordering::Relaxed) > 0 {
        base = base.max(FATOR_SOB_PRESSAO);
    }
    let novos = BMP_BYTES_NOVOS.load(Ordering::Relaxed);
    let vistos = BMP_BYTES_VISTOS.swap(novos, Ordering::Relaxed);
    let mut pendente = f64::from_bits(DIVIDA_BITMAP.load(Ordering::Relaxed));
    pendente += novos.saturating_sub(vistos) as f64 / BYTES_POR_CICLO * TRABALHO_POR_OBJETO;
    let pagar = pendente.min(TETO_POR_QUADRO);
    DIVIDA_BITMAP.store((pendente - pagar).to_bits(), Ordering::Relaxed);
    base + pagar
}
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


/// Diagnostico: tamanhos de tipos centrais do AVM2 (para ler o histograma de blocos).
pub fn tamanhos_n7() -> String {
    use std::mem::size_of;
    format!(
        "ScriptObjectData={} Value={} Object={} usize={}",
        size_of::<crate::avm2::object::ScriptObjectData<'static>>(),
        size_of::<crate::avm2::Value<'static>>(),
        size_of::<crate::avm2::Object<'static>>(),
        size_of::<usize>(),
    )
}

// ===== Compactacao de imagens paradas (Naruto Legaxy, como o "purge" do Flash 10.1) =====
// Imagem com pixels proprios que ficou parada (sem acesso da CPU) por `QUADROS_OCIOSA` quadros
// tem os pixels compactados (deflate rapido); quem desenha usa a textura que ja esta na GPU.
// O 1o acesso aos pixels descompacta sem o jogo perceber. Imagem de simbolo sem escrita so
// solta os pixels (sao decodificados de novo da origem).

/// Quadros sem acesso da CPU para uma imagem ser compactada (padrao ~4 s a 30 fps; ajustavel pela
/// pagina via `n7_compactacao`, ex.: iPhone usa menos quadros e mais orcamento).
pub static QUADROS_OCIOSA_CFG: AtomicU64 = AtomicU64::new(120);
/// Tamanho minimo (bytes de pixels) para valer a pena.
pub const MINIMO_COMPACTAR: usize = 64 * 1024;
/// Bytes de pixels examinados para compactar por quadro (limite de tempo de CPU; ajustavel).
pub static ORCAMENTO_COMPACTAR_CFG: AtomicU64 = AtomicU64::new(2 * 1024 * 1024);

#[inline]
pub fn quadros_ociosa() -> u64 {
    QUADROS_OCIOSA_CFG.load(Ordering::Relaxed)
}
#[inline]
pub fn orcamento_compactar() -> usize {
    ORCAMENTO_COMPACTAR_CFG.load(Ordering::Relaxed) as usize
}
/// Ajusta a compactacao (0 ou negativo mantem). Devolve (quadros, orcamento em bytes).
pub fn definir_compactacao(quadros: f64, orcamento: f64) -> (u64, u64) {
    if quadros >= 1.0 {
        QUADROS_OCIOSA_CFG.store(quadros as u64, Ordering::Relaxed);
    }
    if orcamento >= 65536.0 {
        ORCAMENTO_COMPACTAR_CFG.store(orcamento as u64, Ordering::Relaxed);
    }
    (quadros_ociosa(), orcamento_compactar() as u64)
}

/// Imagens compactadas, bytes de pixels antes e depois, descompactacoes e simbolos soltos.
pub static COMPACTADAS: AtomicU64 = AtomicU64::new(0);
pub static COMPACTADAS_ANTES: AtomicU64 = AtomicU64::new(0);
pub static COMPACTADAS_DEPOIS: AtomicU64 = AtomicU64::new(0);
pub static DESCOMPACTADAS: AtomicU64 = AtomicU64::new(0);
pub static SIMBOLOS_SOLTOS: AtomicU64 = AtomicU64::new(0);
/// Imagens na lista de vigia.
pub static VIGIADAS: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn quadro_atual() -> u64 {
    QUADROS_EXECUTADOS.load(Ordering::Relaxed)
}

/// Imagens do Loader lidas no formato compacto do JPEG (sem decodificar para RGBA).
pub static LEITURAS_COMPACTAS: AtomicU64 = AtomicU64::new(0);

/// Composicao da memoria no instante do maior pico: [peq, med, gde, bmp proprios, etiquetas 0..7,
/// ByteArray, quadro].
pub static FOTO_PICO: [AtomicU64; 14] = [const { AtomicU64::new(0) }; 14];

#[cold]
fn foto_do_pico() {
    let r = Ordering::Relaxed;
    for k in 0..3 {
        FOTO_PICO[k].store(HEAP_CLASSE[k].load(r) as u64, r);
    }
    FOTO_PICO[3].store(BMP_BYTES.load(r), r);
    for k in 0..8 {
        FOTO_PICO[4 + k].store(GRANDES_POR_ETIQUETA[k].load(r) as u64, r);
    }
    FOTO_PICO[12].store(BYTEARRAY_BYTES.load(r) as u64, r);
    FOTO_PICO[13].store(QUADROS_EXECUTADOS.load(r), r);
}

/// Objetos de GC criados por tipo (ruffle_common::contagem_n7).
pub fn criados(i: usize) -> u64 {
    ruffle_common::contagem_n7::criados(i)
}
