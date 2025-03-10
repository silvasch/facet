use jiter::NumberInt;
use shapely::Shape;

#[cfg(feature = "log")]
use log::{error, trace, warn};

#[cfg(not(feature = "log"))]
macro_rules! error {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "log"))]
macro_rules! trace {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "log"))]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

pub fn from_json(target: *mut u8, schema: Shape, json: &str) -> Result<(), String> {
    use jiter::Jiter;
    use shapely::{MapShape, Scalar, ShapeKind};

    trace!("Starting JSON deserialization");
    let mut jiter = Jiter::new(json.as_bytes());

    fn deserialize_value(jiter: &mut Jiter, target: *mut u8, schema: &Shape) -> Result<(), String> {
        trace!("Deserializing value with schema:\n{:?}", schema);
        match &schema.shape {
            ShapeKind::Scalar(scalar) => {
                match scalar {
                    Scalar::String => {
                        trace!("Deserializing String");
                        let s = jiter.next_str().map_err(|e| e.to_string())?;
                        unsafe {
                            *(target as *mut String) = s.to_string();
                        }
                        trace!("Deserialized String: {}", s);
                    }
                    Scalar::U64 => {
                        trace!("Deserializing U64");
                        let n = jiter.next_int().map_err(|e| e.to_string())?;
                        let NumberInt::Int(n) = n;
                        unsafe {
                            *(target as *mut u64) = n as u64;
                        }
                        trace!("Deserialized U64: {}", n);
                    }
                    // Add other scalar types as needed
                    _ => {
                        warn!("Unsupported scalar type: {:?}", scalar);
                        return Err(format!("Unsupported scalar type: {:?}", scalar));
                    }
                }
            }
            ShapeKind::Map(MapShape {
                fields,
                manipulator,
                ..
            }) => {
                trace!("Deserializing Map");
                let first_key = jiter.next_object().map_err(|e| e.to_string())?;
                if let Some(mut key) = first_key {
                    loop {
                        trace!("Processing map key: {}", key);
                        if let Some(field) = fields.iter().find(|f| f.name == key) {
                            let field_schema = (field.schema)();
                            trace!("Deserializing field: {}", field.name);
                            unsafe {
                                manipulator.set_field_raw(target, *field, &mut |field_ptr| {
                                    deserialize_value(jiter, field_ptr, &field_schema).unwrap();
                                });
                            }
                        } else {
                            warn!("Unknown field: {}, skipping", key);
                            // Skip unknown field
                            jiter.next_skip().map_err(|e| e.to_string())?;
                        }
                        if let Some(next_key) = jiter.next_key().map_err(|e| e.to_string())? {
                            key = next_key;
                        } else {
                            break;
                        }
                    }
                } else {
                    trace!("Empty object encountered");
                }
                trace!("Finished deserializing Map");
            }
            // Add support for other shapes (Array, Transparent) as needed
            _ => {
                error!("Unsupported shape: {:?}", schema.shape);
                return Err(format!("Unsupported shape: {:?}", schema.shape));
            }
        }
        Ok(())
    }

    let result = deserialize_value(&mut jiter, target, &schema);
    if result.is_ok() {
        trace!("JSON deserialization completed successfully");
    } else {
        error!("JSON deserialization failed: {:?}", result);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    use log::{Level, Metadata, Record};
    use shapely::Shapely;

    struct SimpleLogger;

    impl log::Log for SimpleLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Info
        }

        fn log(&self, record: &Record) {
            if self.enabled(record.metadata()) {
                println!("{} - {}", record.level(), record.args());
            }
        }

        fn flush(&self) {}
    }

    #[test]
    fn test_from_json() {
        log::set_logger(&SimpleLogger).unwrap();
        log::set_max_level(log::LevelFilter::Trace);

        #[derive(Debug, PartialEq)]
        struct TestStruct {
            name: String,
            age: u64,
        }

        impl Shapely for TestStruct {
            fn shape() -> Shape {
                use shapely::{MapField, MapShape, Shape, ShapeKind, StructManipulator};

                static NAME_FIELD: MapField = MapField {
                    name: "name",
                    schema: <String as Shapely>::shape,
                };
                static AGE_FIELD: MapField = MapField {
                    name: "age",
                    schema: <u64 as Shapely>::shape,
                };
                static SCHEMA: Shape = Shape {
                    name: "TestStruct",
                    size: std::mem::size_of::<TestStruct>(),
                    align: std::mem::align_of::<TestStruct>(),
                    shape: ShapeKind::Map(MapShape {
                        fields: &[NAME_FIELD, AGE_FIELD],
                        open_ended: false,
                        manipulator: &StructManipulator {
                            fields: &[
                                (NAME_FIELD, std::mem::offset_of!(TestStruct, name)),
                                (AGE_FIELD, std::mem::offset_of!(TestStruct, age)),
                            ],
                        },
                    }),
                    display: None,
                    debug: None,
                    set_to_default: None,
                };
                SCHEMA
            }
        }

        let json = r#"{"name": "Alice", "age": 30}"#;
        let mut test_struct = TestStruct {
            name: String::new(),
            age: 0,
        };

        let result = from_json(
            &mut test_struct as *mut TestStruct as *mut u8,
            TestStruct::shape(),
            json,
        );

        assert!(result.is_ok());
        assert_eq!(
            test_struct,
            TestStruct {
                name: "Alice".to_string(),
                age: 30
            }
        );
    }
}
