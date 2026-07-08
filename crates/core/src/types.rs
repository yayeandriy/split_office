use serde::{Deserialize, Serialize};

/// Simplified data types that cover all Parquet/Arrow primitives we expose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Boolean,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Utf8,
    LargeUtf8,
    Date32,
    Date64,
    TimestampMillis,
    TimestampMicros,
    Binary,
    List(Box<DataType>),
    Null,
    Other(String),
}

impl DataType {
    /// Whether this type is numeric (useful for rendering alignment).
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64
                | DataType::Float32
                | DataType::Float64
        )
    }

    /// Short display label for the type.
    pub fn label(&self) -> &str {
        match self {
            DataType::Boolean => "bool",
            DataType::Int8 => "i8",
            DataType::Int16 => "i16",
            DataType::Int32 => "i32",
            DataType::Int64 => "i64",
            DataType::UInt8 => "u8",
            DataType::UInt16 => "u16",
            DataType::UInt32 => "u32",
            DataType::UInt64 => "u64",
            DataType::Float32 => "f32",
            DataType::Float64 => "f64",
            DataType::Utf8 | DataType::LargeUtf8 => "str",
            DataType::Date32 | DataType::Date64 => "date",
            DataType::TimestampMillis | DataType::TimestampMicros => "ts",
            DataType::Binary => "bytes",
            DataType::List(_) => "list",
            DataType::Null => "null",
            DataType::Other(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Convert from Arrow DataType to our simplified DataType.
impl From<&arrow::datatypes::DataType> for DataType {
    fn from(dt: &arrow::datatypes::DataType) -> Self {
        use arrow::datatypes::DataType as A;
        match dt {
            A::Boolean => DataType::Boolean,
            A::Int8 => DataType::Int8,
            A::Int16 => DataType::Int16,
            A::Int32 => DataType::Int32,
            A::Int64 => DataType::Int64,
            A::UInt8 => DataType::UInt8,
            A::UInt16 => DataType::UInt16,
            A::UInt32 => DataType::UInt32,
            A::UInt64 => DataType::UInt64,
            A::Float32 => DataType::Float32,
            A::Float64 => DataType::Float64,
            A::Utf8 => DataType::Utf8,
            A::LargeUtf8 => DataType::LargeUtf8,
            A::Date32 => DataType::Date32,
            A::Date64 => DataType::Date64,
            A::Timestamp(arrow::datatypes::TimeUnit::Millisecond, _) => {
                DataType::TimestampMillis
            }
            A::Timestamp(arrow::datatypes::TimeUnit::Microsecond, _) => {
                DataType::TimestampMicros
            }
            A::Binary | A::LargeBinary => DataType::Binary,
            A::List(field) => DataType::List(Box::new(DataType::from(field.data_type()))),
            A::Null => DataType::Null,
            other => DataType::Other(format!("{other:?}")),
        }
    }
}
