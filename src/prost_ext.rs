pub(crate) trait IntoProstValue {
    fn into_prost(self) -> prost_types::Value;
}

impl IntoProstValue for String {
    fn into_prost(self) -> prost_types::Value {
        prost_types::Value {
            kind: Some(prost_types::value::Kind::StringValue(self)),
        }
    }
}

impl IntoProstValue for f64 {
    fn into_prost(self) -> prost_types::Value {
        prost_types::Value {
            kind: Some(prost_types::value::Kind::NumberValue(self)),
        }
    }
}

impl IntoProstValue for Vec<prost_types::Value> {
    fn into_prost(self) -> prost_types::Value {
        prost_types::Value {
            kind: Some(prost_types::value::Kind::ListValue(
                prost_types::ListValue { values: self },
            )),
        }
    }
}
