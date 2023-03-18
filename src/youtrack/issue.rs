use crate::youtrack::client::Client;
use anyhow::Result;
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueShort {
    id_readable: String,
    summary: String,
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
        Cow::Borrowed(&self.id_readable)
    }
}

impl<'a> Client<'a> {
    pub async fn get_issue_by_id(&self, id: &str) -> Result<IssueShort> {
        self.get(format!("/api/issues/{id}?fields=idReadable,summary"))
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
}
