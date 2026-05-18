use crate::ids::about_id::AboutId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeArguments {
    about: AboutId,
}

impl WakeArguments {
    pub fn new(about: AboutId) -> Self {
        Self { about }
    }

    pub fn about(&self) -> &AboutId {
        &self.about
    }
}
