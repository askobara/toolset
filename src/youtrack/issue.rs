use anyhow::Result;
use recap::Recap;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::borrow::Cow;
use struct_field_names_as_array::FieldNamesAsArray;
use tinytemplate::TinyTemplate;

use crate::{youtrack::Client, normalize::normalize_field_names};

use super::{project::Project, user::User, custom_field::IssueCustomField, tag::Tag};

pub trait BaseIssue {
    fn id(&self) -> Cow<str>;
    fn id_readable(&self) -> Cow<str>;
    fn summary(&self) -> Cow<str>;
}

pub trait YoutrackFields {
    fn fields() -> String;
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssueCustomFields(Vec<IssueCustomField>);

impl IssueCustomFields {
    pub fn get(&self, key: &str) -> Option<&IssueCustomField> {
        for item in self.0.iter() {
            if item.name == key {
                return Some(item);
            }
        }

        None
    }
}

#[derive(Debug, Deserialize, Serialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct IssueShort {
    id: String,
    id_readable: String,
    summary: String,
}

#[derive(Debug, Deserialize, Serialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct IssueLong {
    id: String,
    id_readable: String,
    summary: String,
    project: Project,
    reporter: Option<User>,
    custom_fields: IssueCustomFields
}

#[derive(Debug, Deserialize, Recap)]
#[recap(regex = r#"(?x)
    (?P<project_id>[A-Z]+)
    -
    (?P<number>\d+)
    (?:-(?P<slug>[\w-]+))?
  "#)]
pub struct BranchNameWithIssueId {
    project_id: String,
    number: u32,
    slug: Option<String>,
}

impl BranchNameWithIssueId {
    pub fn short_name(&self) -> String {
        format!("{}-{}", &self.project_id, &self.number)
    }
}

impl BaseIssue for IssueShort {
    fn id(&self) -> Cow<str> {
        Cow::Borrowed(&self.id)
    }

    fn id_readable(&self) -> Cow<str> {
        Cow::Borrowed(&self.id_readable)
    }

    fn summary(&self) -> Cow<str> {
        Cow::Borrowed(&self.summary)
    }
}

impl YoutrackFields for IssueShort {
    fn fields() -> String {
        normalize_field_names(&Self::FIELD_NAMES_AS_ARRAY)
    }
}

impl IssueShort {
    pub fn as_local_branch_name(&self) -> Cow<str> {
        let result = normalize_str_as_branch_name(&self.summary);

        if result.is_empty() {
            Cow::Borrowed(&self.id_readable)
        } else {
            Cow::Owned(format!("{}-{}", &self.id_readable, &result))
        }
    }

    pub fn as_remote_branch_name(&self) -> Cow<str> {
        self.id_readable()
    }

    pub fn is_backend_sub_issue(&self) -> bool {
        self.summary().starts_with("[BE]")
    }
}

impl BaseIssue for IssueLong {
    fn id(&self) -> Cow<str> {
        Cow::Borrowed(&self.id)
    }

    fn id_readable(&self) -> Cow<str> {
        Cow::Borrowed(&self.id_readable)
    }

    fn summary(&self) -> Cow<str> {
        Cow::Borrowed(&self.summary)
    }
}

impl YoutrackFields for IssueLong {
    fn fields() -> String {
        normalize_field_names(&Self::FIELD_NAMES_AS_ARRAY).replace(
            "project",
            &format!("project({})", Project::fields())
        ).replace(
            "reporter",
            &format!("reporter({})", User::fields())
        ).replace(
            "customFields",
            &format!("customFields({})", IssueCustomField::fields())
        )
    }
}

impl<'a> Client<'a> {
    pub async fn get_issue_by_id<T, S>(&self, id: S) -> Result<T>
    where
        T: DeserializeOwned + YoutrackFields,
        S: Into<String> + std::fmt::Display
    {
        let fields = T::fields();

        self.http_client.get(format!("/api/issues/{id}?fields={fields}"))
            .await
    }

    pub async fn get_sub_issues<T, S>(&self, id: S) -> Result<Vec<T>>
    where
        T: DeserializeOwned + YoutrackFields,
        S: Into<String> + std::fmt::Display
    {
        let fields = T::fields();

        self.http_client.get(format!("/api/issues/{id}/links/90-3s/issues?fields={fields}"))
            .await
    }

    pub async fn create_subtask(&self, parent: &IssueLong) -> Result<IssueShort> {
        dbg!(parent);
        let mut tiny = TinyTemplate::new();
        tiny.add_template("be_subtask.md", include_str!("templates/be_subtask.md"))?;

        #[derive(Serialize)]
        struct TinyContext {}
        let context = TinyContext {};

        let me = self.me().await?;

        let body = serde_json::json!({
            "summary": format!("[BE] {}", parent.summary),
            "description": tiny.render("be_subtask.md", &context).unwrap(),
            "project": parent.project,
            "assignee": {
                "id": me.id(),
                "$type": "User",
            },
            "customFields": [
                // parent.custom_fields.get("Type")
                {
                    "id": "94-60",
                    "name": "Type",
                    "$type": "SingleEnumIssueCustomField",
                    "value": {
                        "name": "Sub-Task",
                        "$type": "EnumBundleElement",
                    }
                },
                parent.custom_fields.get("Service"),
                parent.custom_fields.get("Priority"),
                parent.custom_fields.get("Team"),
                parent.custom_fields.get("F.Team"),
                // parent.custom_fields.get("Stage"),
            ]
        });

        dbg!(&body);

        self.http_client.post(format!("/api/issues?fields={fields}", fields = IssueShort::fields()), &body)
            .await
    }

    pub async fn link_issues<T, C>(&self, parent: &T, child: &C) -> Result<IssueShort> 
    where
        T: BaseIssue,
        C: BaseIssue
    {
        #[derive(Debug, Serialize)]
        struct Body {
            id: String
        }

        let body = Body {
            id: child.id().to_string(),
        };

        self.http_client.post(format!("/api/issues/{parent_id}/links/90-3s/issues?fields={fields}", parent_id = parent.id(), fields = IssueShort::fields()), &body)
            .await
    }

    pub async fn add_tag_to_issue(&self, issue: &impl BaseIssue, tag: &Tag) -> Result<Tag> {
        #[derive(Debug, Serialize)]
        struct Body {
            id: String
        }

        let body = Body {
            id: tag.id.clone(),
        };

        self.http_client.post(format!("/api/issues/{id}/tags?fields=id,name", id = issue.id()), &body)
            .await
    }
}

fn normalize_str_as_branch_name(str: &str) -> String {
    let cb = |ref c| !char::is_ascii_alphanumeric(c);

    str.trim_matches(cb)
        .chars()
        .fold(String::with_capacity(str.len()), |mut acc, c| {
            if !cb(c) {
                acc.push(c);
            } else if !acc.ends_with("-") {
                acc.push('-');
            }

            acc
        })
}

#[cfg(test)]
mod tests {
    use super::normalize_str_as_branch_name;

    #[test]
    fn normalize_str_as_branch_name_test() {
        let str = "[[TEST]  (%)Name  of SOME task!!!]";

        let result = normalize_str_as_branch_name(&str);

        assert_eq!(result, "TEST-Name-of-SOME-task");
    }

    #[test]
    fn branch_name_with_issue_id_test() {
        use super::BranchNameWithIssueId;

        let result = "TEST-123-some-name".parse::<BranchNameWithIssueId>();

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.project_id, "TEST");
        assert_eq!(result.number, 123);
        assert_eq!(result.slug, Some("some-name".to_owned()));
    }
}
