use core::ptr::NonNull;
use facet_core::{EnumDef, EnumRepr, Facet, FieldError, Opaque, OpaqueUninit, Shape, VariantKind};

use super::{ISet, Poke, PokeValue};

/// Represents an enum before a variant has been selected
pub struct PokeEnumNoVariant<'mem> {
    data: OpaqueUninit<'mem>,
    shape: &'static Shape,
    def: EnumDef,
}

impl<'mem> PokeEnumNoVariant<'mem> {
    /// Coerce back into a `PokeValue`
    #[inline(always)]
    pub fn into_value(self) -> PokeValue<'mem> {
        unsafe { PokeValue::new(self.data, self.shape) }
    }

    /// Shape getter
    #[inline(always)]
    pub fn shape(&self) -> &'static Shape {
        self.shape
    }
    /// Creates a new PokeEnumNoVariant from raw data
    ///
    /// # Safety
    ///
    /// The data buffer must match the size and alignment of the enum shape described by shape
    pub(crate) unsafe fn new(
        data: OpaqueUninit<'mem>,
        shape: &'static Shape,
        def: EnumDef,
    ) -> Self {
        Self { data, shape, def }
    }

    /// Sets the variant of an enum by name.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No variant with the given name exists.
    pub fn set_variant_by_name(self, variant_name: &str) -> Result<PokeEnum<'mem>, FieldError> {
        let variant_index = self
            .def
            .variants
            .iter()
            .enumerate()
            .find(|(_, v)| v.name == variant_name)
            .map(|(i, _)| i)
            .ok_or(FieldError::NoSuchStaticField)?;

        self.set_variant_by_index(variant_index)
    }

    /// Sets the variant of an enum by index.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The index is out of bounds.
    pub fn set_variant_by_index(self, variant_index: usize) -> Result<PokeEnum<'mem>, FieldError> {
        if variant_index >= self.def.variants.len() {
            return Err(FieldError::IndexOutOfBounds);
        }

        // Get the current variant info
        let variant = &self.def.variants[variant_index];

        // Prepare memory for the enum
        unsafe {
            // Zero out the memory first to ensure clean state
            core::ptr::write_bytes(self.data.as_mut_bytes(), 0, self.shape.layout.size());

            // Set up the discriminant (tag)
            // For enums in Rust, the first bytes contain the discriminant
            let discriminant_value = match &variant.discriminant {
                // If we have an explicit discriminant, use it
                Some(discriminant) => *discriminant,
                // Otherwise, use the variant index directly
                None => variant_index as i64,
            };

            // Write the discriminant value based on the representation
            match self.def.repr {
                EnumRepr::U8 => {
                    let tag_ptr = self.data.as_mut_bytes();
                    *tag_ptr = discriminant_value as u8;
                }
                EnumRepr::U16 => {
                    let tag_ptr = self.data.as_mut_bytes() as *mut u16;
                    *tag_ptr = discriminant_value as u16;
                }
                EnumRepr::U32 => {
                    let tag_ptr = self.data.as_mut_bytes() as *mut u32;
                    *tag_ptr = discriminant_value as u32;
                }
                EnumRepr::U64 => {
                    let tag_ptr = self.data.as_mut_bytes() as *mut u64;
                    *tag_ptr = discriminant_value as u64;
                }
                EnumRepr::USize => {
                    let tag_ptr = self.data.as_mut_bytes() as *mut usize;
                    *tag_ptr = discriminant_value as usize;
                }
                EnumRepr::I8 => {
                    let tag_ptr = self.data.as_mut_bytes() as *mut i8;
                    *tag_ptr = discriminant_value as i8;
                }
                EnumRepr::I16 => {
                    let tag_ptr = self.data.as_mut_bytes() as *mut i16;
                    *tag_ptr = discriminant_value as i16;
                }
                EnumRepr::I32 => {
                    let tag_ptr = self.data.as_mut_bytes() as *mut i32;
                    *tag_ptr = discriminant_value as i32;
                }
                EnumRepr::I64 => {
                    let tag_ptr = self.data.as_mut_bytes() as *mut i64;
                    *tag_ptr = discriminant_value;
                }
                EnumRepr::ISize => {
                    let tag_ptr = self.data.as_mut_bytes() as *mut isize;
                    *tag_ptr = discriminant_value as isize;
                }
                EnumRepr::Default => {
                    // Use a heuristic based on the number of variants
                    if self.def.variants.len() <= 256 {
                        // Can fit in a u8
                        let tag_ptr = self.data.as_mut_bytes();
                        *tag_ptr = discriminant_value as u8;
                    } else if self.def.variants.len() <= 65536 {
                        // Can fit in a u16
                        let tag_ptr = self.data.as_mut_bytes() as *mut u16;
                        *tag_ptr = discriminant_value as u16;
                    } else {
                        // Default to u32
                        let tag_ptr = self.data.as_mut_bytes() as *mut u32;
                        *tag_ptr = discriminant_value as u32;
                    }
                }
                _ => {
                    panic!("Unsupported enum representation: {:?}", self.def.repr);
                }
            }
        }

        // Create PokeEnum with the selected variant
        Ok(PokeEnum {
            data: self.data,
            iset: Default::default(),
            shape: self.shape,
            def: self.def,
            selected_variant: variant_index,
        })
    }
}

/// Allows poking an enum with a selected variant (setting fields, etc.)
pub struct PokeEnum<'mem> {
    data: OpaqueUninit<'mem>,
    iset: ISet,
    shape: &'static Shape,
    def: EnumDef,
    selected_variant: usize,
}

impl<'mem> PokeEnum<'mem> {
    /// Returns the currently selected variant index
    pub fn selected_variant_index(&self) -> usize {
        self.selected_variant
    }

    /// Get a field writer for a field in the currently selected variant.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The field name doesn't exist in the selected variant.
    /// - The selected variant is a unit variant (which has no fields).
    pub fn variant_field_by_name<'s>(&'s mut self, name: &str) -> Result<Poke<'s>, FieldError> {
        let variant = &self.def.variants[self.selected_variant];

        // Find the field in the variant
        match &variant.kind {
            VariantKind::Unit => {
                // Unit variants have no fields
                Err(FieldError::NoSuchStaticField)
            }
            VariantKind::Tuple { fields } => {
                // For tuple variants, find the field by name
                let field = fields
                    .iter()
                    .find(|f| f.name == name)
                    .ok_or(FieldError::NoSuchStaticField)?;

                // Get the field's address
                let field_data = unsafe { self.data.field_uninit(field.offset) };
                let poke = unsafe { Poke::unchecked_new(field_data, field.shape) };
                Ok(poke)
            }
            VariantKind::Struct { fields } => {
                // For struct variants, find the field by name
                let field = fields
                    .iter()
                    .find(|f| f.name == name)
                    .ok_or(FieldError::NoSuchStaticField)?;

                // Get the field's address
                let field_data = unsafe { self.data.field_uninit(field.offset) };
                let poke = unsafe { Poke::unchecked_new(field_data, field.shape) };
                Ok(poke)
            }
            _ => {
                panic!("Unsupported enum variant kind: {:?}", variant.kind);
            }
        }
    }

    /// Marks a field in the current variant as initialized.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the field is not already initialized.
    pub unsafe fn mark_field_as_initialized(&mut self, field_index: usize) {
        self.iset.set(field_index);
    }

    /// Checks if all required fields in the enum are initialized.
    ///
    /// # Panics
    ///
    /// Panics if any field in the selected variant is not initialized.
    pub fn assert_all_fields_initialized(&self) {
        let variant = &self.def.variants[self.selected_variant];

        // Check if all fields of the selected variant are initialized
        match &variant.kind {
            VariantKind::Unit => {
                // Unit variants don't have fields, so they're always fully initialized
            }
            VariantKind::Tuple { fields } | VariantKind::Struct { fields } => {
                // Check each field
                for (field_index, field) in fields.iter().enumerate() {
                    if !self.iset.has(field_index) {
                        panic!(
                            "Field '{}' of variant '{}' was not initialized. Complete schema:\n{}",
                            field.name, variant.name, self.shape
                        );
                    }
                }
            }
            _ => {
                panic!("Unsupported enum variant kind: {:?}", variant.kind);
            }
        }
    }

    fn assert_matching_shape<T: Facet>(&self) {
        if !self.shape.is_type::<T>() {
            panic!(
                "This is a partial \x1b[1;34m{}\x1b[0m, you can't build a \x1b[1;32m{}\x1b[0m out of it",
                self.shape,
                T::SHAPE,
            );
        }
    }

    /// Asserts that every field in the selected variant has been initialized and forgets the PokeEnum.
    ///
    /// This method is only used when the origin is borrowed.
    /// If this method is not called, all fields will be freed when the PokeEnum is dropped.
    ///
    /// # Panics
    ///
    /// This function will panic if any required field is not initialized.
    pub fn build_in_place(self) -> Opaque<'mem> {
        // ensure all fields are initialized
        self.assert_all_fields_initialized();
        let data = unsafe { self.data.assume_init() };
        // prevent field drops when the PokeEnum is dropped
        core::mem::forget(self);
        data
    }

    /// Builds a value of type `T` from the PokeEnum.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - Not all fields in the selected variant have been initialized.
    /// - The generic type parameter T does not match the shape that this PokeEnum is building.
    pub fn build<T: Facet>(self) -> T {
        self.assert_all_fields_initialized();
        self.assert_matching_shape::<T>();

        let result = unsafe {
            let ptr = self.data.as_bytes() as *const T;
            core::ptr::read(ptr)
        };
        core::mem::forget(self);
        result
    }

    /// Build that PokeEnum into a boxed completed shape.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - Not all fields in the selected variant have been initialized.
    /// - The generic type parameter T does not match the shape that this PokeEnum is building.
    pub fn build_boxed<T: Facet>(self) -> Box<T> {
        self.assert_all_fields_initialized();
        self.assert_matching_shape::<T>();

        let boxed = unsafe { Box::from_raw(self.data.as_mut_bytes() as *mut T) };
        core::mem::forget(self);
        boxed
    }

    /// Moves the contents of this `PokeEnum` into a target memory location.
    ///
    /// # Safety
    ///
    /// The target pointer must be valid and properly aligned,
    /// and must be large enough to hold the value.
    /// The caller is responsible for ensuring that the target memory is properly deallocated
    /// when it's no longer needed.
    pub unsafe fn move_into(self, target: NonNull<u8>) {
        self.assert_all_fields_initialized();
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.data.as_mut_bytes(),
                target.as_ptr(),
                self.shape.layout.size(),
            );
        }
        core::mem::forget(self);
    }
}

impl Drop for PokeEnum<'_> {
    fn drop(&mut self) {
        let variant = &self.def.variants[self.selected_variant];

        // Drop fields based on the variant kind
        match &variant.kind {
            VariantKind::Unit => {
                // Unit variants have no fields to drop
            }
            VariantKind::Tuple { fields } | VariantKind::Struct { fields } => {
                // Drop each initialized field
                for (field_index, field) in fields.iter().enumerate() {
                    if self.iset.has(field_index) {
                        if let Some(drop_fn) = field.shape.vtable.drop_in_place {
                            unsafe {
                                drop_fn(self.data.field_init(field.offset));
                            }
                        }
                    }
                }
            }
            _ => {
                panic!("Unsupported enum variant kind: {:?}", variant.kind);
            }
        }
    }
}

/// All possible errors when getting a variant by index or by name
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VariantError {
    /// `variant_by_index` was called with an index that is out of bounds.
    IndexOutOfBounds,

    /// `variant_by_name` or `variant_by_index` was called on a non-enum type.
    NotAnEnum,

    /// `variant_by_name` was called with a name that doesn't match any variant.
    NoSuchVariant,
}

impl std::error::Error for VariantError {}

impl core::fmt::Display for VariantError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VariantError::IndexOutOfBounds => write!(f, "Variant index out of bounds"),
            VariantError::NotAnEnum => write!(f, "Not an enum"),
            VariantError::NoSuchVariant => write!(f, "No such variant"),
        }
    }
}
