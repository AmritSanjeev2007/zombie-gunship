/// Human-readable label, shown in the debug UI.
#[derive(Clone, Debug)]
pub struct Name(pub String);

impl Name {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}
