use jtd::Type;

#[derive(Debug)]
pub struct InferredNumber {
    min: f64,
    max: f64,
    int: bool,
}

impl InferredNumber {
    pub fn new() -> Self {
        Self {
            min: f64::MAX,
            max: f64::MIN,
            int: true,
        }
    }

    pub fn infer(&self, n: f64) -> Self {
        Self {
            min: self.min.min(n),
            max: self.max.max(n),
            int: self.int && n.fract() == 0.0,
        }
    }

    pub fn into_type(&self, default: &NumType) -> Type {
        if self.contained_by(default) {
            return default.into_type();
        }

        let types = [
            NumType::Uint8,
            NumType::Int8,
            NumType::Uint16,
            NumType::Int16,
            NumType::Uint32,
            NumType::Int32,
        ];

        for type_ in &types {
            if self.contained_by(type_) {
                return type_.into_type();
            }
        }

        return NumType::Float64.into_type();
    }

    fn contained_by(&self, type_: &NumType) -> bool {
        if !self.int && !type_.is_float() {
            return false;
        }

        let (min, max) = type_.as_range();
        min <= self.min && max >= self.max
    }
}

/// A type of number to infer by default.
///
/// See [`Hints`][`crate::Hints`] for how this enum is used.
#[derive(Clone)]
pub enum NumType {
    /// Corresponds to [`jtd::Type::Int8`].
    Int8,

    /// Corresponds to [`jtd::Type::Uint8`].
    Uint8,

    /// Corresponds to [`jtd::Type::Int16`].
    Int16,

    /// Corresponds to [`jtd::Type::Uint16`].
    Uint16,

    /// Corresponds to [`jtd::Type::Int32`].
    Int32,

    /// Corresponds to [`jtd::Type::Uint32`].
    Uint32,

    /// Corresponds to [`jtd::Type::Float32`].
    Float32,

    /// Corresponds to [`jtd::Type::Float64`].
    Float64,
}

impl NumType {
    fn is_float(&self) -> bool {
        match self {
            Self::Float32 | Self::Float64 => true,
            _ => false,
        }
    }

    fn as_range(&self) -> (f64, f64) {
        match self {
            Self::Int8 => (i8::MIN as f64, i8::MAX as f64),
            Self::Uint8 => (u8::MIN as f64, u8::MAX as f64),
            Self::Int16 => (i16::MIN as f64, i16::MAX as f64),
            Self::Uint16 => (u16::MIN as f64, u16::MAX as f64),
            Self::Int32 => (i32::MIN as f64, i32::MAX as f64),
            Self::Uint32 => (u32::MIN as f64, u32::MAX as f64),
            Self::Float32 | Self::Float64 => (f64::MIN, f64::MAX),
        }
    }

    fn into_type(&self) -> Type {
        match self {
            Self::Int8 => Type::Int8,
            Self::Uint8 => Type::Uint8,
            Self::Int16 => Type::Int16,
            Self::Uint16 => Type::Uint16,
            Self::Int32 => Type::Int32,
            Self::Uint32 => Type::Uint32,
            Self::Float32 => Type::Float32,
            Self::Float64 => Type::Float64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inferred_number() {
        let n = InferredNumber::new();

        // At first, default always honored.
        assert_eq!(Type::Uint8, n.into_type(&NumType::Uint8));
        assert_eq!(Type::Int8, n.into_type(&NumType::Int8));
        assert_eq!(Type::Uint16, n.into_type(&NumType::Uint16));
        assert_eq!(Type::Int16, n.into_type(&NumType::Int16));
        assert_eq!(Type::Uint32, n.into_type(&NumType::Uint32));
        assert_eq!(Type::Int32, n.into_type(&NumType::Int32));
        assert_eq!(Type::Float32, n.into_type(&NumType::Float32));
        assert_eq!(Type::Float64, n.into_type(&NumType::Float64));

        // Test expanding to limits of uint8.
        let n = InferredNumber::new()
            .infer(u8::MIN as f64)
            .infer(u8::MAX as f64);

        assert_eq!(Type::Uint8, n.into_type(&NumType::Uint8));
        assert_eq!(Type::Uint8, n.into_type(&NumType::Int8));
        assert_eq!(Type::Uint16, n.into_type(&NumType::Uint16));
        assert_eq!(Type::Int16, n.into_type(&NumType::Int16));
        assert_eq!(Type::Uint32, n.into_type(&NumType::Uint32));
        assert_eq!(Type::Int32, n.into_type(&NumType::Int32));
        assert_eq!(Type::Float32, n.into_type(&NumType::Float32));
        assert_eq!(Type::Float64, n.into_type(&NumType::Float64));

        // Test expanding to limits of int8.
        let n = InferredNumber::new()
            .infer(i8::MIN as f64)
            .infer(i8::MAX as f64);

        assert_eq!(Type::Int8, n.into_type(&NumType::Uint8));
        assert_eq!(Type::Int8, n.into_type(&NumType::Int8));
        assert_eq!(Type::Int8, n.into_type(&NumType::Uint16));
        assert_eq!(Type::Int16, n.into_type(&NumType::Int16));
        assert_eq!(Type::Int8, n.into_type(&NumType::Uint32));
        assert_eq!(Type::Int32, n.into_type(&NumType::Int32));
        assert_eq!(Type::Float32, n.into_type(&NumType::Float32));
        assert_eq!(Type::Float64, n.into_type(&NumType::Float64));

        // Test including a non-integer.
        let n = InferredNumber::new().infer(0.5);
        assert_eq!(Type::Float64, n.into_type(&NumType::Uint8));
        assert_eq!(Type::Float64, n.into_type(&NumType::Int8));
        assert_eq!(Type::Float64, n.into_type(&NumType::Uint16));
        assert_eq!(Type::Float64, n.into_type(&NumType::Int16));
        assert_eq!(Type::Float64, n.into_type(&NumType::Uint32));
        assert_eq!(Type::Float64, n.into_type(&NumType::Int32));
        assert_eq!(Type::Float32, n.into_type(&NumType::Float32));
        assert_eq!(Type::Float64, n.into_type(&NumType::Float64));
    }
}
