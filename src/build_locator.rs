use std::fmt;

#[derive(Debug, Default, Builder)]
#[builder(default)]
pub struct BuildLocator<'a> {
    id: Option<i32>,
    user: Option<&'a str>,
    build_type: Option<String>, // TODO: remove owning
    count: Option<u8>,
    branch: Option<&'a str>,
    personal: Option<bool>,
    default_filter: Option<bool>,
}

impl<'a> fmt::Display for BuildLocator<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut locators: Vec<String> = Vec::new();

        if let Some(default_filter) = self.default_filter {
            locators.push(format!("defaultFilter:{default_filter}"));
        }

        if let Some(personal) = self.personal {
            locators.push(format!("personal:{personal}"));
        }

        if let Some(id) = &self.id {
            locators.push(format!("id:{id}"));
        }

        if let Some(user) = &self.user {
            locators.push(format!("user:{user}"));
        }

        if let Some(build_type) = &self.build_type {
            locators.push(format!("buildType:{build_type}"));
        }

        if let Some(count) = &self.count.or(Some(5)) {
            locators.push(format!("count:{count}"));
        }

        if let Some(branch) = &self.branch {
            if *branch != "any" {
                locators.push(format!("branch:{branch}"));
            } else {
                locators.push("branch:default:any".to_string());
            }
        }

        write!(f, "{}", locators.join(","))
    }
}
