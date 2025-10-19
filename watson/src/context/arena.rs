use rustc_hash::FxHashMap;
use std::{hash::Hash, marker::PhantomData, sync::Mutex};
use typed_arena::Arena;
use ustr::Ustr;

pub struct PlainArena<Data, Handle> {
    arena: Arena<Data>,
    handle: PhantomData<Handle>,
}

impl<'ctx, Data, Handle> PlainArena<Data, Handle> {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            handle: PhantomData,
        }
    }

    pub fn alloc(&'ctx self, data: Data) -> Handle
    where
        Handle: InternerHandle<'ctx, Data> + Copy,
    {
        let ptr = self.arena.alloc(data);
        InternerHandle::from_ref(ptr)
    }
}

pub struct InternedArena<Data, Handle> {
    arena: Arena<Data>,
    cache: Mutex<FxHashMap<Data, Handle>>,
}

impl<'ctx, Data, Handle> InternedArena<Data, Handle> {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            cache: Mutex::new(FxHashMap::default()),
        }
    }

    pub fn intern(&'ctx self, data: Data) -> Handle
    where
        Data: Hash + Eq + Clone,
        Handle: InternerHandle<'ctx, Data> + Copy,
    {
        let mut cache = self.cache.lock().unwrap();
        if let Some(handle) = cache.get(&data) {
            *handle
        } else {
            let ptr = self.arena.alloc(data.clone());
            let handle = InternerHandle::from_ref(ptr);
            cache.insert(data, handle);
            handle
        }
    }
}

pub struct NamedArena<Data, Handle> {
    arena: Arena<Data>,
    by_name: Mutex<FxHashMap<Ustr, Handle>>,
}

impl<'ctx, Data, Handle> NamedArena<Data, Handle> {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            by_name: Mutex::new(FxHashMap::default()),
        }
    }

    pub fn alloc(&'ctx self, name: Ustr, data: Data) -> Handle
    where
        Handle: InternerHandle<'ctx, Data> + Copy,
    {
        let ptr = self.arena.alloc(data);
        let handle = InternerHandle::from_ref(ptr);
        self.by_name.lock().unwrap().insert(name, handle);
        handle
    }

    pub fn get(&self, name: Ustr) -> Option<Handle>
    where
        Handle: Copy,
    {
        self.by_name.lock().unwrap().get(&name).copied()
    }
}

pub trait InternerHandle<'ctx, Data> {
    fn from_ref(r: &'ctx Data) -> Self;
}

#[macro_export]
macro_rules! generate_arena_handle {
    ($handle:ident<$ctx:lifetime> => $data:ty) => {
        #[derive(std::fmt::Debug, std::clone::Clone, std::marker::Copy, std::cmp::Eq)]
        pub struct $handle<$ctx>(pub &$ctx $data);

        impl<$ctx> std::cmp::PartialEq for $handle<$ctx> {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::addr_eq(self.0, other.0)
            }
        }

        impl<$ctx> std::cmp::PartialOrd for $handle<$ctx> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<$ctx> std::cmp::Ord for $handle<$ctx> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                (self.0 as *const $data as usize).cmp(&(other.0 as *const $data as usize))
            }
        }

        impl<$ctx> std::hash::Hash for $handle<$ctx> {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                std::ptr::hash(self.0, state);
            }
        }

        impl<$ctx> std::ops::Deref for $handle<$ctx> {
            type Target = $data;

            fn deref(&self) -> &'ctx Self::Target {
                self.0
            }
        }

        impl<$ctx> $crate::context::arena::InternerHandle<$ctx, $data> for $handle<$ctx> {
            fn from_ref(r: &'ctx $data) -> Self {
                Self(r)
            }
        }
    };
    ($handle:ident => $data:ident) => {
        generate_arena_handle!($handle<'ctx> => $data);
    };
}
