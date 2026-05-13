pub(crate) fn heap_bytes(len: usize) -> usize {
    len
}

pub(crate) fn base_size<T>() -> usize {
    core::mem::size_of::<T>()
}
