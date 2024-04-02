use crate::lang::function::Function;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    String(String),
    Function(Function),
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(x) => write!(f, "{}", x),
            Value::Number(x) => write!(f, "{}", x),
            Value::Nil => write!(f, "nil"),
            Value::String(s) => write!(f, "{}", s),
            Value::Function(func) => {
                if func.native {
                    write!(f, "<native fn>")
                } else {
                    write!(f, "<fn {}>", func.name)
                }
            }
        }
    }
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Nil => true,
            Value::Bool(b) => !b,
            _ => false,
        }
    }

    pub fn eq(&self, other: &Value) -> bool {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            return false;
        }

        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Nil, _) => true,
            _ => unreachable!("Unrecognized value equality comparison"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn true_is_not_falsey() {
        assert_eq!(Value::Bool(true).is_falsey(), false);
    }

    #[test]
    fn false_is_falsey() {
        assert_eq!(Value::Bool(false).is_falsey(), true);
    }

    #[test]
    fn nil_is_falsey() {
        assert_eq!(Value::Nil.is_falsey(), true);
    }

    #[test]
    fn numbers_are_not_falsey() {
        assert_eq!(Value::Number(3.14).is_falsey(), false);
    }

    #[test]
    fn nil_equals_nil() {
        let a = Value::Nil;
        let b = Value::Nil;
        assert_eq!(a.eq(&b), true);
    }

    #[test]
    fn equal_numbers_are_equal() {
        let a = Value::Number(25.9);
        let b = Value::Number(25.9);
        assert_eq!(a.eq(&b), true);
    }

    #[test]
    fn different_numbers_are_not_equal() {
        let a = Value::Number(0.0);
        let b = Value::Number(25.9);
        assert_eq!(a.eq(&b), false);
    }

    #[test]
    fn different_types_are_not_equal() {
        let a = Value::Number(0.0);
        let b = Value::Bool(false);
        assert_eq!(a.eq(&b), false);
    }

    #[test]
    fn different_strings_are_not_equal() {
        let a = Value::String(String::from("star wars"));
        let b = Value::String(String::from("star trek"));
        assert_eq!(a.eq(&b), false);
    }

    #[test]
    fn equal_strings_are_equal() {
        let a = Value::String(String::from("topaz is neat!"));
        let b = Value::String(String::from("topaz is neat!"));
        assert_eq!(a.eq(&b), true);
    }
}
