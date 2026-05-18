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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_the_about_it_was_built_with() {
        let about = AboutId::parse("about:1").unwrap();
        let args = WakeArguments::new(about.clone());
        assert_eq!(args.about(), &about);
    }
}
