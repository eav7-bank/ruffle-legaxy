//! Ruffle Legaxy: pilha "sombra" das funcoes AS3 em execucao, para um perfilador por amostragem
//! (o medidor do PC ou o app) saber QUAL codigo do jogo segura o quadro. Desligada por padrao: com
//! `ATIVO` falso o custo e uma leitura atomica por chamada.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub const MAX: usize = 256;

pub static ATIVO: AtomicBool = AtomicBool::new(false);
/// Profundidade atual (pode passar de MAX; so as MAX primeiras ficam guardadas).
pub static PROFUNDIDADE: AtomicUsize = AtomicUsize::new(0);
/// Identificador (endereco do metodo) de cada nivel.
pub static PILHA: [AtomicUsize; MAX] = [const { AtomicUsize::new(0) }; MAX];

static NOMES: Mutex<Option<HashMap<usize, String>>> = Mutex::new(None);

#[inline]
pub fn ativo() -> bool {
    ATIVO.load(Ordering::Relaxed)
}

/// Guarda o nome na primeira vez que o metodo aparece (o chamador so monta o nome se precisar).
pub fn conhece(id: usize) -> bool {
    NOMES.lock().ok().and_then(|g| g.as_ref().map(|m| m.contains_key(&id))).unwrap_or(false)
}

pub fn registrar_nome(id: usize, nome: String) {
    if let Ok(mut g) = NOMES.lock() {
        g.get_or_insert_with(HashMap::new).insert(id, nome);
    }
}

pub fn nome(id: usize) -> String {
    NOMES
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|m| m.get(&id).cloned()))
        .unwrap_or_else(|| format!("?{id:x}"))
}

#[inline]
pub fn entrar(id: usize) {
    let p = PROFUNDIDADE.fetch_add(1, Ordering::Relaxed);
    if p < MAX {
        PILHA[p].store(id, Ordering::Relaxed);
    }
}

#[inline]
pub fn sair() {
    let _ = PROFUNDIDADE.try_update(Ordering::Relaxed, Ordering::Relaxed, |p| p.checked_sub(1));
}

/// Copia a pilha atual (do mais externo ao mais interno) — chamada pela thread do perfilador.
pub fn copiar(saida: &mut Vec<usize>) {
    saida.clear();
    let p = PROFUNDIDADE.load(Ordering::Relaxed).min(MAX);
    for i in 0..p {
        saida.push(PILHA[i].load(Ordering::Relaxed));
    }
}
