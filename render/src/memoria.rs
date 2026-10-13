//! Ruffle Legaxy: bytes de texturas de bitmap vivas na GPU (somados pelo renderizador), para o
//! `System.totalMemory` refletir tambem a memoria de video — no celular ela sai da mesma RAM.

use std::sync::atomic::{AtomicU64, Ordering};

static BYTES_TEXTURAS: AtomicU64 = AtomicU64::new(0);
static TEXTURAS_VIVAS: AtomicU64 = AtomicU64::new(0);

pub fn bytes_texturas() -> u64 {
    BYTES_TEXTURAS.load(Ordering::Relaxed)
}

pub fn texturas_vivas() -> u64 {
    TEXTURAS_VIVAS.load(Ordering::Relaxed)
}

/// Guarde junto da textura: conta ao criar e desconta ao largar.
#[derive(Debug)]
pub struct ContaTextura(u64);

impl ContaTextura {
    pub fn new(largura: u32, altura: u32) -> Self {
        let b = largura as u64 * altura as u64 * 4;
        BYTES_TEXTURAS.fetch_add(b, Ordering::Relaxed);
        TEXTURAS_VIVAS.fetch_add(1, Ordering::Relaxed);
        Self(b)
    }
}

impl Drop for ContaTextura {
    fn drop(&mut self) {
        BYTES_TEXTURAS.fetch_sub(self.0, Ordering::Relaxed);
        TEXTURAS_VIVAS.fetch_sub(1, Ordering::Relaxed);
    }
}
