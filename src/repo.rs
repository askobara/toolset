use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug)]
pub struct BranchNameMeta {
    pub refname: String,
    pub local_name: String,
    pub upstream_name: Option<String>,
    pub oid: git2::Oid,
    pub summary: Option<String>,
}

pub struct Repo {
    repo: git2::Repository
}

impl Repo  {
    pub fn new(path: Option<&Path>) -> Result<Self> {
        let path = crate::normalize::normalize_path(path)?;
        let repo = git2::Repository::discover(path)?;

        Ok(Self { repo })
    }

    pub fn get_name(&self, remote_name: Option<&str>) -> Result<String> {
        let remote = self.repo.find_remote(remote_name.unwrap_or("origin"))?;
        let url = remote.url().context("Remote url contains non-utf8 symbols")?;

        crate::normalize::normalize(url)
            .context("Cannot get repo name")
    }

    pub fn get_branch_name_meta(&self, branch_name: Option<&str>) -> Result<BranchNameMeta> {
        match branch_name {
            Some(_) => todo!(),
            None => {
                let head = self.repo.head()?;

                if !head.is_branch() {
                    anyhow::bail!("HEAD is not a branch");
                }

                let oid = head.target().context("HEAD is symbolic one")?;
                let summary = self.repo.find_commit(oid).ok().and_then(|c| c.summary().map(ToOwned::to_owned));

                let refname = head.name().context("unable to get a branch name due to non-utf8 symbols")?;

                let upstream_name: Option<String> = self.repo.branch_upstream_name(refname)
                    .ok()
                    .and_then(|b| b.as_str().map(ToOwned::to_owned))
                    .and_then(crate::normalize::normalize)
                ;

                Ok(BranchNameMeta {
                    refname: refname.to_owned(),
                    local_name: crate::normalize::normalize(refname).context("Non utf-8")?,
                    upstream_name,
                    oid,
                    summary,
                })
            }
        }
    }

    pub fn normalize_branch_name(&self, branch_name: Option<&str>) -> Result<String> {
        match branch_name {
            Some(bn) => Ok(bn.into()),
            None => {
                let head = self.repo.head()?;

                let refname = head.name().context("unable to get a branch name due to non-utf8 symbols")?;

                let r = self.repo.branch_upstream_name(refname)
                    .ok()
                    .and_then(|b| b.as_str().map(String::from))
                    .unwrap_or(refname.to_owned())
                ;

                crate::normalize::normalize(&r)
                    .context("Cannot get branch name")
            }
        }
    }

    pub fn count_ahead_commits(&self) -> Result<usize> {
        let mut revwalk = self.repo.revwalk()?;
        // TODO: get default branch name
        revwalk.push_range("origin/master..HEAD")?;

        Ok(revwalk.count())
    }

    pub fn set_upstream(&self, local_name: &str, remote_name: &str, id: git2::Oid) -> Result<()> {
        let mut b = self.repo.find_branch(local_name, git2::BranchType::Local)?;

        self.repo.reference(
            format!("refs/remotes/origin/{remote_name}").as_str(),
            id,
            true,
            ""
        )?;

        b.set_upstream(Some(format!("origin/{remote_name}").as_str()))?;

        Ok(())
    }

    pub fn push(&self, refname: &str, remote_name: &str) -> Result<()> {
        let mut options = Self::get_push_options();

        let name = format!("{}:refs/heads/{}", refname, remote_name);

        dbg!(&name);

        self.repo.find_remote("origin")?.push(
            &[name.as_str()],
            Some(&mut options)
        )?;

        Ok(())
    }

    fn get_push_options() -> git2::PushOptions<'static> {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            git2::Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                Path::new(
                    &format!("{}/.ssh/id_rsa", std::env::var("HOME").unwrap())
                ),
                None,
            )
        });

        callbacks.push_update_reference(|ref_name, status| {
            dbg!(ref_name, status);
            Ok(())
        });

        let mut options = git2::PushOptions::new();
        options.remote_callbacks(callbacks);

        options
    }
}
