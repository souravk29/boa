use std::cell::Cell;

use boa_gc::GcRefCell;
use boa_macros::{Finalize, Trace};

use crate::{
    JsString,
    object::{
        JsObject,
        shape::{Shape, WeakShape, slot::Slot},
    },
};

#[cfg(test)]
mod tests;

/// An inline cache entry for a property access.
#[derive(Clone, Debug, Trace, Finalize)]
pub(crate) struct InlineCache {
    /// The property that is accessed.
    pub(crate) name: JsString,

    /// A pointer is kept to the shape to avoid the shape from being deallocated.
    /// This is the shape of the object being accessed.
    pub(crate) shape: GcRefCell<WeakShape>,

    /// A weak reference to the prototype object where the property was found.
    /// This is `None` if the property is an own property.
    /// For transitive prototype properties, this stores the actual prototype object
    /// that contains the property (could be multiple levels up the chain).
    pub(crate) prototype: GcRefCell<Option<JsObject>>,

    /// The [`Slot`] of the property.
    #[unsafe_ignore_trace]
    pub(crate) slot: Cell<Slot>,
}

impl InlineCache {
    pub(crate) const fn new(name: JsString) -> Self {
        Self {
            name,
            shape: GcRefCell::new(WeakShape::None),
            prototype: GcRefCell::new(None),
            slot: Cell::new(Slot::new()),
        }
    }

    /// Set the inline cache for an own property.
    pub(crate) fn set(&self, shape: &Shape, slot: Slot) {
        *self.shape.borrow_mut() = shape.into();
        *self.prototype.borrow_mut() = None;
        self.slot.set(slot);
    }

    /// Set the inline cache for a prototype property (including transitive prototypes).
    ///
    /// # Arguments
    ///
    /// * `object_shape` - The shape of the object being accessed.
    /// * `prototype` - The prototype object where the property was found.
    /// * `slot` - The slot where the property is stored in the prototype.
    pub(crate) fn set_prototype(&self, object_shape: &Shape, prototype: &JsObject, slot: Slot) {
        *self.shape.borrow_mut() = object_shape.into();
        *self.prototype.borrow_mut() = Some(prototype.clone());
        self.slot.set(slot);
    }

    pub(crate) fn slot(&self) -> Slot {
        self.slot.get()
    }

    /// Returns true, if the [`InlineCache`]'s shape matches with the given shape.
    ///
    /// Otherwise we reset the internal weak reference to [`WeakShape::None`],
    /// so it can be deallocated by the GC.
    ///
    /// This method is for own properties only (not prototype properties).
    pub(crate) fn match_or_reset(&self, shape: &Shape) -> Option<(Shape, Slot)> {
        let mut old = self.shape.borrow_mut();

        let old_upgraded = old.upgrade();
        if old_upgraded.as_ref().map_or(0, Shape::to_addr_usize) == shape.to_addr_usize() {
            // Check if this is an own property (no prototype cached)
            if self.prototype.borrow().is_none() {
                return old_upgraded.map(|shape| (shape, self.slot()));
            }
        }

        *old = WeakShape::None;
        drop(old);
        *self.prototype.borrow_mut() = None;
        None
    }

    /// Returns the prototype object and slot if the inline cache matches for a prototype property.
    ///
    /// This method validates both the object shape and the prototype object to ensure
    /// the entire prototype chain is still valid for transitive prototype caching.
    ///
    /// # Arguments
    ///
    /// * `object_shape` - The shape of the object being accessed.
    ///
    /// # Returns
    ///
    /// Returns `Some((prototype_object, slot))` if the cache is valid, where `prototype_object`
    /// is the actual prototype object containing the property. Returns `None` if the cache
    /// is invalid or if the property is not a prototype property.
    pub(crate) fn match_prototype_or_reset(
        &self,
        object_shape: &Shape,
    ) -> Option<(JsObject, Slot)> {
        let mut cached_shape = self.shape.borrow_mut();
        let mut cached_proto = self.prototype.borrow_mut();

        // Try to upgrade the cached object shape
        let cached_shape_upgraded = cached_shape.upgrade();

        // Check if object shape matches
        if cached_shape_upgraded
            .as_ref()
            .map_or(0, Shape::to_addr_usize)
            != object_shape.to_addr_usize()
        {
            // Object shape mismatch, reset cache
            *cached_shape = WeakShape::None;
            *cached_proto = None;
            return None;
        }

        // If we have a cached prototype object, return it
        if let Some(proto) = cached_proto.as_ref() {
            return Some((proto.clone(), self.slot()));
        }

        // No prototype cached
        None
    }
}
