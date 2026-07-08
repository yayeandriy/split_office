use arrow::array::{
    Array, BooleanArray, Date32Array, Float32Array, Float64Array, Int16Array, Int32Array,
    Int64Array, Int8Array, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
};
use arrow::record_batch::RecordBatch;

/// Format a single cell value as a display string.
///
/// Returns `"null"` for null values.
pub fn format_cell(batch: &RecordBatch, col: usize, row: usize) -> String {
    if col >= batch.num_columns() || row >= batch.num_rows() {
        return String::new();
    }

    let array = batch.column(col);

    if array.is_null(row) {
        return "null".to_string();
    }

    macro_rules! downcast_fmt {
        ($ty:ty) => {{
            let arr = array.as_any().downcast_ref::<$ty>().unwrap();
            format!("{}", arr.value(row))
        }};
    }

    use arrow::datatypes::DataType;
    match array.data_type() {
        DataType::Boolean => {
            let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
            if arr.value(row) { "true" } else { "false" }.to_string()
        }
        DataType::Int8 => downcast_fmt!(Int8Array),
        DataType::Int16 => downcast_fmt!(Int16Array),
        DataType::Int32 => downcast_fmt!(Int32Array),
        DataType::Int64 => downcast_fmt!(Int64Array),
        DataType::UInt8 => downcast_fmt!(UInt8Array),
        DataType::UInt16 => downcast_fmt!(UInt16Array),
        DataType::UInt32 => downcast_fmt!(UInt32Array),
        DataType::UInt64 => downcast_fmt!(UInt64Array),
        DataType::Float32 => {
            let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
            format!("{:.4}", arr.value(row))
        }
        DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
            format!("{:.4}", arr.value(row))
        }
        DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
            arr.value(row).to_string()
        }
        DataType::Date32 => {
            let arr = array.as_any().downcast_ref::<Date32Array>().unwrap();
            // Days since epoch → ISO date string.
            let days = arr.value(row);
            chrono_days_to_date(days)
        }
        _ => format!("{:?}", array.data_type()),
    }
}

fn chrono_days_to_date(days: i32) -> String {
    // Simple epoch calculation without pulling in chrono.
    // epoch = 1970-01-01, days since then.
    let total_days = days as i64;
    // Rough ISO date string.
    let year = 1970 + total_days / 365;
    let doy = total_days % 365;
    let month = (doy / 30).clamp(0, 11) + 1;
    let day = (doy % 30) + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}
