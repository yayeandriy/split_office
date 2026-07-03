use core::{Column, DataType, Schema};
use polars::prelude::Schema as PolarsSchema;
use polars::datatypes::DataType as PolarsDataType;

/// Convert a Polars schema to our core Schema type.
pub fn polars_schema_to_core(schema: &PolarsSchema) -> Schema {
    let columns = schema
        .iter()
        .map(|(name, dtype)| Column::new(name.as_str(), polars_dtype_to_core(dtype)))
        .collect();
    Schema { columns }
}

fn polars_dtype_to_core(dt: &PolarsDataType) -> DataType {
    match dt {
        PolarsDataType::Boolean => DataType::Boolean,
        PolarsDataType::Int8 => DataType::Int8,
        PolarsDataType::Int16 => DataType::Int16,
        PolarsDataType::Int32 => DataType::Int32,
        PolarsDataType::Int64 => DataType::Int64,
        PolarsDataType::UInt8 => DataType::UInt8,
        PolarsDataType::UInt16 => DataType::UInt16,
        PolarsDataType::UInt32 => DataType::UInt32,
        PolarsDataType::UInt64 => DataType::UInt64,
        PolarsDataType::Float32 => DataType::Float32,
        PolarsDataType::Float64 => DataType::Float64,
        PolarsDataType::String => DataType::Utf8,
        PolarsDataType::Date => DataType::Date32,
        PolarsDataType::Datetime(polars::datatypes::TimeUnit::Milliseconds, _) => {
            DataType::TimestampMillis
        }
        PolarsDataType::Datetime(polars::datatypes::TimeUnit::Microseconds, _) => {
            DataType::TimestampMicros
        }
        PolarsDataType::Binary => DataType::Binary,
        PolarsDataType::List(inner) => DataType::List(Box::new(polars_dtype_to_core(inner))),
        PolarsDataType::Null => DataType::Null,
        other => DataType::Other(format!("{other:?}")),
    }
}
