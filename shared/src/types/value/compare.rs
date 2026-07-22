//! Equality, ordering and hashing for [`Value`].
//!
//! Note: `PartialEq` is implemented manually while `Hash` is written by hand on
//! top of the `Display` representation — the pairing is intentional and must be
//! kept in sync.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::Value;

// =============================================================================
//         SECTION: TRAIT IMPLEMENTATIONS
// =============================================================================

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (
                Value::Range {
                    start: s1,
                    end: e1,
                    inclusive: i1,
                    step: p1,
                },
                Value::Range {
                    start: s2,
                    end: e2,
                    inclusive: i2,
                    step: p2,
                },
            ) => s1 == s2 && e1 == e2 && i1 == i2 && p1 == p2,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Pair(a1, a2), Value::Pair(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Triple(a1, a2, a3), Value::Triple(b1, b2, b3)) => {
                a1 == b1 && a2 == b2 && a3 == b3
            }
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Option(a), Value::Option(b)) => a == b,
            (Value::Result(a), Value::Result(b)) => a == b,
            (Value::Pointer(a), Value::Pointer(b)) => a == b,
            (
                Value::Reference {
                    target: t1,
                    mutable: m1,
                },
                Value::Reference {
                    target: t2,
                    mutable: m2,
                },
            ) => t1 == t2 && m1 == m2,
            (
                Value::Enum {
                    name: n1,
                    variant: v1,
                    data: d1,
                },
                Value::Enum {
                    name: n2,
                    variant: v2,
                    data: d2,
                },
            ) => n1 == n2 && v1 == v2 && d1 == d2,
            (
                Value::Object {
                    type_id: t1,
                    fields: f1,
                },
                Value::Object {
                    type_id: t2,
                    fields: f2,
                },
            ) => t1 == t2 && f1 == f2,
            (
                Value::NativeObject {
                    type_id: t1,
                    object: o1,
                    ..
                },
                Value::NativeObject {
                    type_id: t2,
                    object: o2,
                    ..
                },
            ) => t1 == t2 && Arc::ptr_eq(o1, o2),
            (Value::Lambda(a), Value::Lambda(b)) => a == b,
            (
                Value::Promise {
                    task_id: t1,
                    status: s1,
                    ..
                },
                Value::Promise {
                    task_id: t2,
                    status: s2,
                    ..
                },
            ) => t1 == t2 && s1 == s2,
            (
                Value::Generator {
                    id: i1, state: s1, ..
                },
                Value::Generator {
                    id: i2, state: s2, ..
                },
            ) => i1 == i2 && s1 == s2,
            (Value::Channel { id: i1, .. }, Value::Channel { id: i2, .. }) => i1 == i2,
            (Value::Null, Value::Null) => true,
            (Value::Undefined, Value::Undefined) => true,
            (Value::Type(a), Value::Type(b)) => a == b,
            (
                Value::Error {
                    message: m1,
                    kind: k1,
                    ..
                },
                Value::Error {
                    message: m2,
                    kind: k2,
                    ..
                },
            ) => m1 == m2 && k1 == k2,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        fn tag(v: &Value) -> u8 {
            match v {
                Value::Number(_) => 0,
                Value::String(_) => 1,
                Value::Boolean(_) => 2,
                Value::Char(_) => 3,
                Value::Array(_) => 4,
                Value::Pair(_, _) => 5,
                Value::Triple(_, _, _) => 6,
                Value::Tuple(_) => 7,
                Value::Set(_) => 8,
                Value::Map(_) => 9,
                Value::Option(_) => 10,
                Value::Result(_) => 11,
                Value::Pointer(_) => 12,
                Value::Reference { .. } => 13,
                Value::Enum { .. } => 14,
                Value::Object { .. } => 15,
                Value::NativeObject { .. } => 16,
                Value::Lambda(_) => 17,
                Value::PartialApp { .. } => 18,
                Value::Promise { .. } => 19,
                Value::Generator { .. } => 20,
                Value::Channel { .. } => 21,
                Value::Null => 22,
                Value::Undefined => 23,
                Value::Type(_) => 24,
                Value::Error { .. } => 25,
                Value::Range { .. } => 26,
                Value::Bytes(_) => 27,
            }
        }

        let d = tag(self).cmp(&tag(other));
        if d != Ordering::Equal {
            return d;
        }
        self.to_string().cmp(&other.to_string())
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        self.to_string().hash(state);
    }
}
