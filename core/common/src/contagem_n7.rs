//! Naruto Legaxy: contagem (cumulativa) de objetos de GC criados, por tipo. Diagnostico de memoria:
//! lido pelo `n7_memoria(57..)` do web. Indices: 0 objeto AS3 (ScriptObjectData), 1 ativacao (newactivation),
//! 2 string, 3 Array, 4 Vector, 5 Function, 6 Method, 7 Multiname, 8 Class, 9 VTable, 10 escopo, 11 Namespace,
//! 12 Script, 13 E4X, 14 ByteArray, 15 Event, 16 XML/XMLList, 17 Date, 18 QName/NamespaceObject, 19 outros.
use std::sync::atomic::{AtomicU64, Ordering};

pub const TIPOS: usize = 24;
pub static CRIADOS: [AtomicU64; TIPOS] = [const { AtomicU64::new(0) }; TIPOS];

#[inline]
pub fn criado(i: usize) {
    CRIADOS[i.min(TIPOS - 1)].fetch_add(1, Ordering::Relaxed);
}

pub fn criados(i: usize) -> u64 {
    CRIADOS.get(i).map(|c| c.load(Ordering::Relaxed)).unwrap_or(0)
}

/// `Gc::new` contado: `gc_contado!(tipo, mc, valor)`.
#[macro_export]
macro_rules! gc_contado {
    ($i:expr, $mc:expr, $v:expr $(,)?) => {{
        $crate::contagem_n7::criado($i);
        ::gc_arena::Gc::new($mc, $v)
    }};
}
