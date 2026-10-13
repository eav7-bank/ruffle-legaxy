//! Ruffle Legaxy: gancho chamado de tempos em tempos DURANTE a execucao de AS3 longa (ex.: a montagem
//! das tabelas de 23 MB do Naruto Legaxy, 1,7 s no PC e 6-9 s num celular). O app Android usa para
//! atender os comandos de ciclo de vida do sistema (pausar, retomar, superficie criada/destruida): a
//! thread de interface do Android fica PARADA esperando esses comandos serem atendidos, e passando de
//! ~5 s o sistema mostra "o app nao esta respondendo". O gancho NAO pode mexer no Player (ele esta em uso).

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static GANCHO: OnceLock<fn()> = OnceLock::new();
static INICIO: OnceLock<Instant> = OnceLock::new();
static ULTIMO_MS: AtomicU64 = AtomicU64::new(0);
/// Intervalo minimo entre chamadas (ms).
pub const INTERVALO_MS: u64 = 50;

/// Registra o gancho (1x por processo).
pub fn definir_gancho(f: fn()) {
    let _ = GANCHO.set(f);
    let _ = INICIO.set(Instant::now());
}

/// Chamado pela execucao de AS3 a cada lote de instrucoes; chama o gancho se ja passou o intervalo.
#[inline]
pub fn talvez_servir() {
    if let Some(f) = GANCHO.get() {
        let agora = INICIO.get().map(|i| i.elapsed().as_millis() as u64).unwrap_or(0);
        if agora.saturating_sub(ULTIMO_MS.load(Ordering::Relaxed)) >= INTERVALO_MS {
            ULTIMO_MS.store(agora, Ordering::Relaxed);
            f();
        }
    }
}
