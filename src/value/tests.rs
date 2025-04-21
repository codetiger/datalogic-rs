//! Unit tests for DataValue extensions.

#[cfg(test)]
mod tests {
    use super::super::*;
    use datavalue_rs::{DataValue, Number};
    use std::cmp::Ordering;

    #[test]
    fn test_truthy_values() {
        // Falsy values
        assert!(!is_truthy(&DataValue::Null));
        assert!(!is_truthy(&DataValue::Bool(false)));
        assert!(!is_truthy(&DataValue::Number(Number::Integer(0))));
        assert!(!is_truthy(&DataValue::Number(Number::Float(0.0))));
        assert!(!is_truthy(&DataValue::String("")));
        assert!(!is_truthy(&DataValue::Array(&[])));

        // Truthy values
        assert!(is_truthy(&DataValue::Bool(true)));
        assert!(is_truthy(&DataValue::Number(Number::Integer(1))));
        assert!(is_truthy(&DataValue::Number(Number::Integer(-1))));
        assert!(is_truthy(&DataValue::Number(Number::Float(0.1))));
        assert!(is_truthy(&DataValue::Number(Number::Float(-0.1))));
        assert!(is_truthy(&DataValue::String("hello")));
        assert!(is_truthy(&DataValue::String("0"))); // String "0" is truthy
        assert!(is_truthy(&DataValue::String("false"))); // String "false" is truthy

        // Non-empty array is truthy
        let array_data = vec![DataValue::Number(Number::Integer(1))];
        let array = DataValue::Array(&array_data);
        assert!(is_truthy(&array));

        // Object is always truthy, even empty
        let empty_object_data: Vec<(&str, DataValue)> = vec![];
        let empty_object = DataValue::Object(&empty_object_data);
        assert!(is_truthy(&empty_object));

        let object_data = vec![("key", DataValue::Number(Number::Integer(1)))];
        let object = DataValue::Object(&object_data);
        assert!(is_truthy(&object));
    }

    #[test]
    fn test_compare_values() {
        // Same type comparisons
        assert_eq!(
            compare_values(
                &DataValue::Number(Number::Integer(1)),
                &DataValue::Number(Number::Integer(2))
            ),
            Ordering::Less
        );
        assert_eq!(
            compare_values(
                &DataValue::Number(Number::Integer(2)),
                &DataValue::Number(Number::Integer(1))
            ),
            Ordering::Greater
        );
        assert_eq!(
            compare_values(
                &DataValue::Number(Number::Integer(2)),
                &DataValue::Number(Number::Integer(2))
            ),
            Ordering::Equal
        );

        assert_eq!(
            compare_values(
                &DataValue::Number(Number::Float(1.1)),
                &DataValue::Number(Number::Float(2.2))
            ),
            Ordering::Less
        );
        assert_eq!(
            compare_values(
                &DataValue::Number(Number::Float(2.2)),
                &DataValue::Number(Number::Float(1.1))
            ),
            Ordering::Greater
        );
        assert_eq!(
            compare_values(
                &DataValue::Number(Number::Float(2.2)),
                &DataValue::Number(Number::Float(2.2))
            ),
            Ordering::Equal
        );

        assert_eq!(
            compare_values(&DataValue::String("a"), &DataValue::String("b")),
            Ordering::Less
        );
        assert_eq!(
            compare_values(&DataValue::String("b"), &DataValue::String("a")),
            Ordering::Greater
        );
        assert_eq!(
            compare_values(&DataValue::String("a"), &DataValue::String("a")),
            Ordering::Equal
        );

        // Arrays
        let array1 = DataValue::Array(&[DataValue::Number(Number::Integer(1))]);
        let array2 = DataValue::Array(&[
            DataValue::Number(Number::Integer(1)),
            DataValue::Number(Number::Integer(2)),
        ]);
        let array3 = DataValue::Array(&[DataValue::Number(Number::Integer(2))]);

        assert_eq!(compare_values(&array1, &array2), Ordering::Less); // Shorter array is less
        assert_eq!(compare_values(&array1, &array3), Ordering::Less); // First element determines order
        assert_eq!(compare_values(&array3, &array2), Ordering::Greater); // First element determines order

        // Different type comparisons
        // Number vs String
        assert_eq!(
            compare_values(
                &DataValue::Number(Number::Integer(1)),
                &DataValue::String("1")
            ),
            Ordering::Equal
        );
        assert_eq!(
            compare_values(
                &DataValue::Number(Number::Integer(1)),
                &DataValue::String("2")
            ),
            Ordering::Less
        );
        assert_eq!(
            compare_values(
                &DataValue::Number(Number::Integer(2)),
                &DataValue::String("1")
            ),
            Ordering::Greater
        );
        assert_eq!(
            compare_values(
                &DataValue::String("1"),
                &DataValue::Number(Number::Integer(1))
            ),
            Ordering::Equal
        );

        // Boolean vs Number
        assert_eq!(
            compare_values(
                &DataValue::Bool(true),
                &DataValue::Number(Number::Integer(1))
            ),
            Ordering::Equal
        );
        assert_eq!(
            compare_values(
                &DataValue::Bool(false),
                &DataValue::Number(Number::Integer(0))
            ),
            Ordering::Equal
        );
        assert_eq!(
            compare_values(
                &DataValue::Bool(true),
                &DataValue::Number(Number::Integer(0))
            ),
            Ordering::Greater
        );
        assert_eq!(
            compare_values(
                &DataValue::Bool(false),
                &DataValue::Number(Number::Integer(1))
            ),
            Ordering::Less
        );

        // Null vs other types
        assert_eq!(
            compare_values(&DataValue::Null, &DataValue::Number(Number::Integer(0))),
            Ordering::Less
        );
        assert_eq!(
            compare_values(&DataValue::Number(Number::Integer(0)), &DataValue::Null),
            Ordering::Greater
        );
        assert_eq!(
            compare_values(&DataValue::Null, &DataValue::Null),
            Ordering::Equal
        );
    }

    #[test]
    fn test_loose_equals() {
        // Same type equality
        assert!(loose_equals(&DataValue::Null, &DataValue::Null));
        assert!(loose_equals(&DataValue::Bool(true), &DataValue::Bool(true)));
        assert!(loose_equals(
            &DataValue::Bool(false),
            &DataValue::Bool(false)
        ));
        assert!(loose_equals(
            &DataValue::Number(Number::Integer(1)),
            &DataValue::Number(Number::Integer(1))
        ));
        assert!(loose_equals(
            &DataValue::Number(Number::Float(1.5)),
            &DataValue::Number(Number::Float(1.5))
        ));
        assert!(loose_equals(
            &DataValue::String("hello"),
            &DataValue::String("hello")
        ));

        // Number to string conversion
        assert!(loose_equals(
            &DataValue::Number(Number::Integer(1)),
            &DataValue::String("1")
        ));
        assert!(loose_equals(
            &DataValue::Number(Number::Float(1.5)),
            &DataValue::String("1.5")
        ));
        assert!(loose_equals(
            &DataValue::String("1"),
            &DataValue::Number(Number::Integer(1))
        ));
        assert!(!loose_equals(
            &DataValue::Number(Number::Integer(1)),
            &DataValue::String("2")
        ));
        assert!(!loose_equals(
            &DataValue::String("hello"),
            &DataValue::Number(Number::Integer(1))
        ));

        // Boolean to number conversion
        assert!(loose_equals(
            &DataValue::Bool(true),
            &DataValue::Number(Number::Integer(1))
        ));
        assert!(loose_equals(
            &DataValue::Bool(false),
            &DataValue::Number(Number::Integer(0))
        ));
        assert!(loose_equals(
            &DataValue::Number(Number::Integer(1)),
            &DataValue::Bool(true)
        ));
        assert!(loose_equals(
            &DataValue::Number(Number::Integer(0)),
            &DataValue::Bool(false)
        ));
        assert!(!loose_equals(
            &DataValue::Bool(true),
            &DataValue::Number(Number::Integer(0))
        ));
        assert!(!loose_equals(
            &DataValue::Bool(false),
            &DataValue::Number(Number::Integer(1))
        ));

        // Boolean to string conversion
        assert!(loose_equals(
            &DataValue::Bool(true),
            &DataValue::String("true")
        ));
        assert!(loose_equals(
            &DataValue::Bool(false),
            &DataValue::String("false")
        ));
        assert!(loose_equals(
            &DataValue::String("true"),
            &DataValue::Bool(true)
        ));
        assert!(loose_equals(
            &DataValue::String("false"),
            &DataValue::Bool(false)
        ));
        assert!(loose_equals(
            &DataValue::Bool(true),
            &DataValue::String("1")
        ));
        assert!(loose_equals(
            &DataValue::Bool(false),
            &DataValue::String("0")
        ));
        assert!(!loose_equals(
            &DataValue::Bool(true),
            &DataValue::String("false")
        ));
        assert!(!loose_equals(
            &DataValue::Bool(false),
            &DataValue::String("true")
        ));

        // Null is not equal to anything else
        assert!(!loose_equals(&DataValue::Null, &DataValue::Bool(false)));
        assert!(!loose_equals(
            &DataValue::Null,
            &DataValue::Number(Number::Integer(0))
        ));
        assert!(!loose_equals(&DataValue::Null, &DataValue::String("")));
    }

    #[test]
    fn test_strict_equals() {
        // Same type and value
        assert!(strict_equals(&DataValue::Null, &DataValue::Null));
        assert!(strict_equals(
            &DataValue::Bool(true),
            &DataValue::Bool(true)
        ));
        assert!(strict_equals(
            &DataValue::Bool(false),
            &DataValue::Bool(false)
        ));
        assert!(strict_equals(
            &DataValue::Number(Number::Integer(1)),
            &DataValue::Number(Number::Integer(1))
        ));
        assert!(strict_equals(
            &DataValue::Number(Number::Float(1.5)),
            &DataValue::Number(Number::Float(1.5))
        ));
        assert!(strict_equals(
            &DataValue::String("hello"),
            &DataValue::String("hello")
        ));

        // Different types but same underlying value should NOT be equal
        assert!(!strict_equals(
            &DataValue::Number(Number::Integer(1)),
            &DataValue::String("1")
        ));
        assert!(!strict_equals(
            &DataValue::Bool(true),
            &DataValue::Number(Number::Integer(1))
        ));
        assert!(!strict_equals(
            &DataValue::Bool(false),
            &DataValue::Number(Number::Integer(0))
        ));
        assert!(!strict_equals(
            &DataValue::Bool(true),
            &DataValue::String("true")
        ));

        // Arrays and objects
        let array1 = DataValue::Array(&[
            DataValue::Number(Number::Integer(1)),
            DataValue::Number(Number::Integer(2)),
        ]);
        let array2 = DataValue::Array(&[
            DataValue::Number(Number::Integer(1)),
            DataValue::Number(Number::Integer(2)),
        ]);
        let array3 = DataValue::Array(&[
            DataValue::Number(Number::Integer(1)),
            DataValue::Number(Number::Integer(3)),
        ]);

        assert!(strict_equals(&array1, &array2)); // Same content
        assert!(!strict_equals(&array1, &array3)); // Different content

        let object1 = DataValue::Object(&[("key", DataValue::Number(Number::Integer(1)))]);
        let object2 = DataValue::Object(&[("key", DataValue::Number(Number::Integer(1)))]);
        let object3 = DataValue::Object(&[("key", DataValue::Number(Number::Integer(2)))]);

        assert!(strict_equals(&object1, &object2)); // Same content
        assert!(!strict_equals(&object1, &object3)); // Different content
    }

    #[test]
    fn test_data_value_ext_trait() {
        // Test is_truthy
        let value1 = DataValue::Bool(true);
        let value2 = DataValue::Number(Number::Integer(0));

        assert!(value1.is_truthy());
        assert!(!value2.is_truthy());

        // Test compare
        let value3 = DataValue::Number(Number::Integer(1));
        let value4 = DataValue::Number(Number::Integer(2));

        assert_eq!(value3.compare(&value4), Ordering::Less);
        assert_eq!(value4.compare(&value3), Ordering::Greater);
        assert_eq!(value3.compare(&value3), Ordering::Equal);

        // Test loose_equals
        let value5 = DataValue::String("1");

        assert!(value3.loose_equals(&value5));
        assert!(!value4.loose_equals(&value5));

        // Test strict_equals
        assert!(!value3.strict_equals(&value5));
        assert!(value3.strict_equals(&value3));
    }
}
