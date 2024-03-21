use std::{
    borrow::Borrow, cell::UnsafeCell, collections::HashSet, marker::PhantomData, mem::ManuallyDrop,
    ptr::NonNull,
};

use compact_str::{format_compact, CompactString};
use fxhash::FxBuildHasher;

use crate::core;

/// Data structure that supports deserialization
/// while retaining state and auxiliary information.
pub(crate) struct SeedData(UnsafeCell<SeedDataInner>);

struct SeedDataInner {
    ids: HashSet<CompactString, FxBuildHasher>,
    parent_ids: Vec<CompactString>,
}

impl SeedData {
    pub fn new() -> Self {
        Self(UnsafeCell::new(SeedDataInner {
            ids: HashSet::with_hasher(FxBuildHasher::default()),
            parent_ids: Vec::with_capacity(8),
        }))
    }
}

impl SeedData {
    // TODO Verify correctness
    // https://www.commonwl.org/v1.2/SchemaSalad.html#Identifier_resolution
    pub fn generate_id(&self, id: CompactString) -> Result<String, String> {
        // SAFETY The dereferencing of the pointer occurs only here
        // and no other method exposes the reference externally.
        let inner = unsafe { &mut *self.0.get() };

        let (id, parent_id) = match (id.strip_prefix('#'), inner.parent_ids.last()) {
            (Some(sub_id), _) => {
                let new_id = CompactString::from(sub_id);
                (new_id.clone(), new_id)
            }
            (None, _) if id.contains(['#', ':']) => {
                let new_id = CompactString::from(id.replace(':', "#"));
                (new_id.clone(), new_id)
            }
            (None, Some(parent_id)) => {
                let new_id = format_compact!("{parent_id}/{id}");
                (new_id, id)
            }
            (None, None) => (id.clone(), id),
        };

        if !inner.ids.contains(id.as_str()) {
            inner.ids.insert(id.clone());
            inner.parent_ids.push(parent_id);
            Ok(id.into_string())
        } else {
            Err(format!("duplicate identifier `{id}`"))
        }
    }

    pub fn push_subscope(&self, subscope: &str) {
        // SAFETY The dereferencing of the pointer occurs only here
        // and no other method exposes the reference externally.
        let inner = unsafe { &mut *self.0.get() };

        match inner.parent_ids.last().cloned() {
            Some(mut id) => {
                id.push('/');
                id.push_str(subscope);
                inner.parent_ids.push(id)
            }
            None => inner.parent_ids.push(CompactString::from(subscope)),
        }
    }

    pub fn pop_parent_id(&self) {
        // SAFETY The dereferencing of the pointer occurs only here
        // and no other method exposes the reference externally.
        let inner = unsafe { &mut *self.0.get() };
        let _ = inner.parent_ids.pop();
    }

    pub fn extend(&self, other: SeedData) -> Result<(), String> {
        // SAFETY The dereferencing of the pointer occurs only here
        // and no other method exposes the reference externally.
        let inner = unsafe { &mut *self.0.get() };
        let SeedDataInner { ids, .. } = other.0.into_inner();

        for id in ids.into_iter() {
            if !inner.ids.contains(id.as_str()) {
                inner.ids.insert(id);
            } else {
                return Err(format!("duplicate identifier `{id}`"));
            }
        }

        Ok(())
    }
}

impl Clone for SeedData {
    fn clone(&self) -> Self {
        // SAFETY The dereferencing of the pointer occurs only here
        // and no other method exposes the reference externally.
        let inner = unsafe { &mut *self.0.get() };
        let inner_clone = SeedDataInner::clone(inner);
        Self(UnsafeCell::new(inner_clone))
    }
}

impl Clone for SeedDataInner {
    fn clone(&self) -> Self {
        Self {
            ids: HashSet::with_capacity_and_hasher(0, FxBuildHasher::default()),
            parent_ids: self.parent_ids.clone(),
        }
    }
}

/// Allows to derive the `DeserializeSeed` corresponding
/// to the object on which the method is invoked.
pub(crate) trait IntoDeserializeSeed<'de, 'sd> {
    type Value: serde::de::DeserializeSeed<'de, Value = Self>;
    fn into_dseed(data: &'sd SeedData) -> Self::Value;
}

impl<'de, 'sd> IntoDeserializeSeed<'de, 'sd> for core::Bool {
    type Value = PhantomData<core::Bool>;

    #[inline]
    fn into_dseed(_: &'sd SeedData) -> Self::Value {
        PhantomData
    }
}

impl<'de, 'sd> IntoDeserializeSeed<'de, 'sd> for core::Int {
    type Value = PhantomData<core::Int>;

    #[inline]
    fn into_dseed(_: &'sd SeedData) -> Self::Value {
        PhantomData
    }
}

impl<'de, 'sd> IntoDeserializeSeed<'de, 'sd> for core::Long {
    type Value = PhantomData<core::Long>;

    #[inline]
    fn into_dseed(_: &'sd SeedData) -> Self::Value {
        PhantomData
    }
}

impl<'de, 'sd> IntoDeserializeSeed<'de, 'sd> for core::Float {
    type Value = PhantomData<core::Float>;

    #[inline]
    fn into_dseed(_: &'sd SeedData) -> Self::Value {
        PhantomData
    }
}

impl<'de, 'sd> IntoDeserializeSeed<'de, 'sd> for core::Double {
    type Value = PhantomData<core::Double>;

    #[inline]
    fn into_dseed(_: &'sd SeedData) -> Self::Value {
        PhantomData
    }
}

impl<'de, 'sd> IntoDeserializeSeed<'de, 'sd> for core::StrValue {
    type Value = PhantomData<core::StrValue>;

    #[inline]
    fn into_dseed(_: &'sd SeedData) -> Self::Value {
        PhantomData
    }
}
