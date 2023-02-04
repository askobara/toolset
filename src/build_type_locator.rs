use std::fmt;
use crate::build_type::BuildType;
use std::convert::From;

#[derive(Debug, Default, Builder, Clone)]
#[builder(default)]
pub struct BuildTypeLocator {
    #[builder(setter(into, strip_option))]
    id: Option<String>,
    #[builder(setter(into, strip_option))]
    r#type: Option<String>,
    #[builder(setter(into, strip_option))]
    name: Option<String>,
    items: Vec<BuildTypeLocator>,
}

impl BuildTypeLocator {
    pub fn only_builds() -> Self {
        Self {
            r#type: Some("regular".into()),
            name: Some("Build".into()),
            ..Self::default()
        }
    }

    pub fn only_deploys() -> Self {
        Self {
            r#type: Some("deployment".into()),
            ..Self::default()
        }
    }
}

impl<'a> fmt::Display for BuildTypeLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut locators: Vec<String> = Vec::new();

        if let Some(id) = &self.id {
            locators.push(format!("id:{id}"));
        }

        if let Some(type_) = &self.r#type {
            locators.push(format!("type:{type_}"));
        }

        if let Some(name) = &self.name {
            locators.push(format!("name:{name}"));
        }

        for l in &self.items {
            locators.push(format!("item:({l})"));
        }

        write!(f, "{}", locators.join(","))
    }
}

impl From<Vec<BuildType>> for BuildTypeLocator {
    fn from(item: Vec<BuildType>) -> Self {
        Self {
            items: item.iter().map(BuildTypeLocator::from).collect::<Vec<_>>(),
            ..Self::default()
        }
    }
}

impl From<&BuildType> for BuildTypeLocator {
    fn from(item: &BuildType) -> Self {
        Self {
            id: Some(item.id.to_owned()),
            ..Self::default()
        }
    }
}

impl From<BuildTypeLocator> for String {
    fn from(item: BuildTypeLocator) -> Self {
        format!("({item})")
    }
}
