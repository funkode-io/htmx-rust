//! A value's fields, read from a URL and printed back as one. The two are
//! inverse: what is printed here is parsed here.

use std::any::type_name;

use axum::extract::RawPathParams;
use form_urlencoded::{Serializer, byte_serialize};
use serde::{
    Serialize,
    de::{DeserializeOwned, Deserializer, Error as _, Visitor, value},
    forward_to_deserialize_any,
};

pub use serde_html_form::de::Error;

/// What a request's URL carries, as one list of fields: the path parameters,
/// then the query string.
pub fn fields(parameters: &RawPathParams, query: Option<&str>) -> String {
    let parameters = Serializer::new(String::new())
        .extend_pairs(parameters)
        .finish();
    format!("{parameters}&{}", query.unwrap_or_default())
}

pub fn parse<T: DeserializeOwned>(fields: &str) -> Result<T, Error> {
    T::deserialize(Fields(serde_html_form::Deserializer::new(
        form_urlencoded::parse(fields.as_bytes()),
    )))
}

/// Checks one view of a chain when it is registered: that every parameter of
/// its `part` of the path has a field of `T` to fill it, and that none of its
/// fields has the name of a field `seen` in an ancestor, since every view on a
/// chain is read from the same fields of the URL.
///
/// # Panics
///
/// If a parameter has no field of that name, a field's name was `seen`, or `T`
/// does not name its fields: it is not a struct, or has a flattened field. The
/// panic stops the app from starting; this is not called while a request is
/// served.
pub fn check_at_startup<T: DeserializeOwned>(
    part: &str,
    seen: &mut Vec<(&'static str, &'static str)>,
) {
    /// Takes the names of the fields a type asks for, and nothing else.
    struct Names<'a>(&'a mut Option<&'static [&'static str]>);

    impl<'de> Deserializer<'de> for Names<'_> {
        type Error = value::Error;

        fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, value::Error> {
            Err(value::Error::custom(""))
        }

        fn deserialize_unit_struct<V: Visitor<'de>>(
            self,
            name: &'static str,
            visitor: V,
        ) -> Result<V::Value, value::Error> {
            self.deserialize_struct(name, &[], visitor)
        }

        fn deserialize_struct<V: Visitor<'de>>(
            self,
            _: &'static str,
            fields: &'static [&'static str],
            _: V,
        ) -> Result<V::Value, value::Error> {
            *self.0 = Some(fields);
            Err(value::Error::custom(""))
        }

        forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit newtype_struct seq tuple tuple_struct map
            enum identifier ignored_any
        }
    }

    let view = type_name::<T>();
    let mut names = None;
    let _ = T::deserialize(Names(&mut names));
    // Stops the app from starting when a view is not a struct with named
    // fields. Runs only when a view is registered, never while a request is
    // served.
    let Some(names) = names else {
        panic!(
            "{view} does not name its fields: a view is a struct with named fields, or none, \
             and none of them flattened"
        )
    };
    let parameters = part.split('{').skip(1);
    for parameter in parameters.filter_map(|rest| Some(rest.split_once('}')?.0)) {
        // Stops the app from starting when a parameter of the path has no
        // field to fill it. Runs only when a view is registered, never while a
        // request is served.
        assert!(
            names.contains(&parameter),
            "{view} has no field for the parameter {{{parameter}}} of its part of the path: {part}"
        );
    }
    for &name in names {
        if let Some((_, ancestor)) = seen.iter().find(|(seen, _)| *seen == name) {
            // Stops the app from starting when a view and one it is nested in
            // have a field of the same name. Runs only when a view is
            // registered, never while a request is served.
            panic!(
                "{view} and {ancestor}, which it is nested in, both have a field named \
                 {name}: a name is used once on the way down"
            );
        }
        seen.push((name, view));
    }
}

/// Adds a view's `part` to the `path` of its ancestors. A last `/` gives way to
/// a part that starts with one, and a path that is still empty is `/`.
pub fn join(path: &mut String, part: &str) {
    if part.starts_with('/') && path.ends_with('/') {
        path.pop();
    }
    path.push_str(part);
    if path.is_empty() {
        path.push('/');
    }
}

/// Prints one view of a chain into the `path` of its chain, which starts as
/// the route the router matches: fills the parameters that the fields of
/// `value` name, and adds its other fields to `query`. So the path printed is
/// the route with its parameters filled, whatever the values.
///
/// # Panics
///
/// If the value is not made of named fields that each hold a plain value or a
/// list of them.
pub fn print<T: Serialize>(value: &T, path: &mut String, query: &mut Serializer<'_, String>) {
    let fields = serde_html_form::to_string(value)
        .unwrap_or_else(|error| panic!("{} does not print as a URL: {error}", type_name::<T>()));
    for (name, value) in form_urlencoded::parse(fields.as_bytes()) {
        let parameter = format!("{{{name}}}");
        if path.contains(&parameter) {
            // In a path a space is `%20`: a `+` there stands for itself.
            let value = byte_serialize(value.as_bytes()).collect::<String>();
            *path = path.replace(&parameter, &value.replace('+', "%20"));
        } else {
            query.append_pair(&name, &value);
        }
    }
}

/// Reads fields as `serde_html_form` does, and also into a type that has none
/// (`struct Home;`), whatever the URL carries.
struct Fields<'de>(serde_html_form::Deserializer<'de>);

impl<'de> Deserializer<'de> for Fields<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.0.deserialize_any(visitor)
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit newtype_struct seq tuple tuple_struct map
        struct enum identifier ignored_any
    }
}
