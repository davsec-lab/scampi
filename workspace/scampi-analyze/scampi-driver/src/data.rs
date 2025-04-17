use std::collections::HashMap;

use log::warn;
use rustc_middle::ty::Ty;
use rustc_span::Span;
use serde::Serialize;

/// Consolidates information about a function.
#[derive(Serialize)]
pub struct FnData {
    /// List of parameters (_not_ arguments).
    parameters: Vec<ParamData>,

    /// The place where this function is defined.
    span: String,
}

/// Consolidates information about a function parameter.
#[derive(Serialize)]
pub struct ParamData {
    /// The type of this parameter.
    ty: String,

    properties: HashMap<&'static str, bool>,
}

/// Consolidates information about a function invocaton.
#[derive(Serialize)]
pub struct InvocData {
    /// The function being invoked.
    function: String,

    /// The place where this invocation occurs.
    span: String,
}

impl FnData {
    pub fn new(parameters: Vec<ParamData>, span: Span) -> Self {
        Self {
            parameters,
            span: clean_span(span),
        }
    }

    // pub fn new(parameters: Vec<ParamData>, span: String) -> Self {
    //     Self {
    //         parameters,
    //         span
    //     }
    // }
}

impl ParamData {
    pub fn new(ty: &Ty) -> Self {
        Self {
            ty: format!("{:?}", ty),
            properties: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: &'static str, value: bool) -> Self {
        self.properties.insert(key, value);
        self
    }
}

impl InvocData {
    pub fn new(function: String, source: Span) -> Self {
        Self {
            function,
            span: clean_span(source),
        }
    }
}

fn clean_span(span: rustc_span::Span) -> String {
    let mut span_string = format!("{:?}", span);

    if let Some(home_dir) = dirs::home_dir() {
        let home_dir_string = home_dir.to_str().unwrap();

        if !span_string.starts_with(home_dir_string) {
            let mut cwd = std::env::current_dir().unwrap();
            cwd.push(span_string);

            span_string = String::from(cwd.to_str().unwrap());
        }
    } else {
        warn!("Could not get home directory.")
    }

    span_string.trim_end_matches(" (#0)").to_owned()
}
